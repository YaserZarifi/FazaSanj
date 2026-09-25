//! One executor per cleanup method. Each one checks the guard again before touching anything.

use std::io::Write;
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use fazasanj_model::{ActionResult, ActionStatus, CleanupAction, CleanupMethod, CleanupOptions, SafetyLevel};

use crate::commands::{KnownCommand, OpenTarget};
use crate::error::CleanupError;
use crate::guard::Guard;
use crate::remove::{self, Ctx, Tally};
use crate::walk::{self, io_problem, Kind};
use crate::win;

/// DISM and friends report "done, restart needed" with this code.
const SUCCESS_REBOOT_REQUIRED: u32 = 3010;

pub(crate) struct Run<'a> {
    pub guard: &'a Guard,
    pub opts: CleanupOptions,
    pub cancel: &'a AtomicBool,
    pub progress: &'a dyn Fn(&Path, u64),
}

pub(crate) fn empty_result(a: &CleanupAction, status: ActionStatus) -> ActionResult {
    ActionResult {
        index: a.index,
        path: a.path.clone(),
        method: a.method,
        status,
        bytes_freed: 0,
        files_removed: 0,
        files_skipped: 0,
        skipped_paths: Vec::new(),
        error: None,
        would_remove: Vec::new(),
    }
}

pub(crate) fn failed(a: &CleanupAction, e: CleanupError) -> ActionResult {
    let status = if e.is_block() {
        ActionStatus::Blocked
    } else if matches!(e, CleanupError::InUse | CleanupError::VhdxInUse) {
        ActionStatus::SkippedInUse
    } else {
        ActionStatus::Failed
    };
    let mut r = empty_result(a, status);
    r.error = Some(e.to_api());
    r
}

pub(crate) fn run_action(a: &CleanupAction, run: &Run) -> ActionResult {
    if a.blocked {
        return failed(a, CleanupError::PlanBlocked);
    }
    match a.method {
        CleanupMethod::Recycle | CleanupMethod::DeleteContents => run_files(a, run),
        CleanupMethod::Command => run_command(a, run),
        CleanupMethod::OpenAppSetting => run_open(a, run),
        CleanupMethod::CompactVhdx => run_compact(a, run),
        CleanupMethod::ManualOnly => empty_result(a, ActionStatus::NeedsManual),
    }
}

/// Permanent only for `safe` items (which only the knowledge base can produce) and only when
/// the user allowed it.
pub(crate) fn is_permanent(a: &CleanupAction, opts: &CleanupOptions) -> bool {
    a.safety == SafetyLevel::Safe && (opts.permanent_for_safe || a.permanent)
}

fn run_files(a: &CleanupAction, run: &Run) -> ActionResult {
    let path = Path::new(&a.path);
    if let Err(e) = run.guard.check(path) {
        return failed(a, e);
    }
    if let Err(e) = std::fs::symlink_metadata(path) {
        return failed(a, io_problem(&e));
    }
    let root = walk::scan(path, run.guard, run.cancel);
    let ctx = Ctx::new(run.guard, run.opts.dry_run, run.cancel, run.progress);
    let permanent = is_permanent(a, &run.opts);
    let mut tally = Tally::default();
    match a.method {
        CleanupMethod::DeleteContents => {
            if root.kind != Kind::Dir {
                return failed(a, root.problem.clone().unwrap_or(CleanupError::NotAFolder));
            }
            if let Some(p) = &root.problem {
                return failed(a, p.clone());
            }
            if permanent {
                remove::delete_permanently(&root, false, &ctx, &mut tally);
            } else {
                for c in &root.children {
                    remove::recycle(c, &ctx, &mut tally);
                }
            }
        }
        _ => {
            if permanent {
                remove::delete_permanently(&root, true, &ctx, &mut tally);
            } else {
                remove::recycle(&root, &ctx, &mut tally);
            }
        }
    }
    files_result(a, tally, run.opts.dry_run)
}

fn files_result(a: &CleanupAction, t: Tally, dry: bool) -> ActionResult {
    let (status, error) = if t.cancelled {
        let s = if t.files_removed > 0 { ActionStatus::Partial } else { ActionStatus::Failed };
        (s, Some(CleanupError::Cancelled))
    } else if dry {
        (ActionStatus::DryRun, t.first_problem.clone())
    } else if t.files_skipped == 0 {
        (ActionStatus::Done, None)
    } else if t.files_removed > 0 {
        (ActionStatus::Partial, t.first_problem.clone())
    } else {
        let p = t.first_problem.clone().unwrap_or(CleanupError::InUse);
        let s = match &p {
            CleanupError::InUse => ActionStatus::SkippedInUse,
            e if e.is_block() => ActionStatus::Blocked,
            _ => ActionStatus::Failed,
        };
        (s, Some(p))
    };
    ActionResult {
        index: a.index,
        path: a.path.clone(),
        method: a.method,
        status,
        bytes_freed: t.bytes,
        files_removed: t.files_removed,
        files_skipped: t.files_skipped,
        skipped_paths: t.skipped_paths,
        error: error.map(|e| e.to_api()),
        would_remove: if dry { t.removed_paths } else { Vec::new() },
    }
}

fn drive_root(path: &str) -> Option<PathBuf> {
    let p = path.strip_prefix("\\\\?\\").unwrap_or(path);
    let b = p.as_bytes();
    (b.len() >= 2 && b[1] == b':' && b[0].is_ascii_alphabetic()).then(|| PathBuf::from(format!("{}:\\", &p[..1])))
}

pub(crate) fn action_drive(a: &CleanupAction) -> Option<PathBuf> {
    if a.method == CleanupMethod::Command {
        if let Some(d) = a.command.as_deref().and_then(KnownCommand::parse).and_then(|k| k.drive()) {
            return Some(PathBuf::from(format!("{d}:\\")));
        }
    }
    drive_root(&a.path)
}

fn run_command(a: &CleanupAction, run: &Run) -> ActionResult {
    // The target (WinSxS, hiberfil.sys...) is on the block list on purpose: the official tool
    // handles it and we never touch the files. The sandbox still applies.
    if let Err(e) = run.guard.check_sandbox(Path::new(&a.path)) {
        return failed(a, e);
    }
    let Some(cmd) = a.command.as_deref().and_then(KnownCommand::parse) else {
        return failed(a, CleanupError::CommandNotAllowed);
    };
    if run.opts.dry_run {
        return empty_result(a, ActionStatus::DryRun);
    }
    let Some(sys) = win::system_dir() else {
        return failed(a, CleanupError::Win32(0));
    };
    let exe = sys.join(cmd.program());
    let params = cmd.args().join(" ");
    let drive = action_drive(a);
    let before = drive.as_deref().and_then(win::free_space);
    (run.progress)(Path::new(&a.path), 0);
    match win::run_and_wait(&exe, &params, a.needs_admin || cmd.needs_admin(), cmd.shows_ui()) {
        Ok(0) | Ok(SUCCESS_REBOOT_REQUIRED) => {
            let after = drive.as_deref().and_then(win::free_space);
            let mut r = empty_result(a, ActionStatus::Done);
            r.bytes_freed = match (before, after) {
                (Some(b), Some(f)) => f.saturating_sub(b),
                _ => 0,
            };
            r
        }
        Ok(code) => failed(a, CleanupError::ExitCode(code)),
        Err(e) => failed(a, e),
    }
}

fn run_open(a: &CleanupAction, run: &Run) -> ActionResult {
    let Some(raw) = a.command.as_deref() else {
        return empty_result(a, ActionStatus::NeedsManual);
    };
    let Some(target) = OpenTarget::parse(raw) else {
        return failed(a, CleanupError::TargetNotAllowed);
    };
    if run.opts.dry_run {
        return empty_result(a, ActionStatus::DryRun);
    }
    let Some(sys) = win::system_dir() else {
        return failed(a, CleanupError::Win32(0));
    };
    let resolved = target.resolve(&sys);
    match win::shell_open(&resolved.to_string_lossy()) {
        Ok(()) => empty_result(a, ActionStatus::OpenedSetting),
        Err(e) => failed(a, e),
    }
}

fn run_compact(a: &CleanupAction, run: &Run) -> ActionResult {
    let path = Path::new(&a.path);
    if let Err(e) = run.guard.check(path) {
        return failed(a, e);
    }
    let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase());
    if !matches!(ext.as_deref(), Some("vhdx") | Some("vhd")) || a.path.chars().any(|c| c == '"' || c.is_control()) {
        return failed(a, CleanupError::NotVhdx);
    }
    let before = match std::fs::metadata(path) {
        Ok(m) if m.is_file() => m.len(),
        Ok(_) => return failed(a, CleanupError::NotVhdx),
        Err(e) => return failed(a, io_problem(&e)),
    };
    if let Err(e) = check_not_in_use(path) {
        return failed(a, e);
    }
    if run.opts.dry_run {
        return empty_result(a, ActionStatus::DryRun);
    }
    match compact(path, run.guard) {
        Ok(()) => {
            let after = std::fs::metadata(path).map(|m| m.len()).unwrap_or(before);
            let mut r = empty_result(a, ActionStatus::Done);
            r.bytes_freed = before.saturating_sub(after);
            r
        }
        Err(e) => failed(a, e),
    }
}

/// Docker Desktop and WSL keep their disks open. An exclusive open fails while they run.
fn check_not_in_use(path: &Path) -> Result<(), CleanupError> {
    match std::fs::OpenOptions::new().read(true).share_mode(0).open(path) {
        Ok(_) => Ok(()),
        Err(e) => Err(match io_problem(&e) {
            CleanupError::InUse => CleanupError::VhdxInUse,
            other => other,
        }),
    }
}

fn compact(path: &Path, guard: &Guard) -> Result<(), CleanupError> {
    let script = format!(
        "select vdisk file=\"{p}\"\r\nattach vdisk readonly\r\ncompact vdisk\r\ndetach vdisk\r\nexit\r\n",
        p = path.display()
    );
    let mut file = tempfile::Builder::new().prefix("fazasanj-compact-").suffix(".txt").tempfile()?;
    // UTF-16 with a BOM so paths with Persian or other non ASCII names survive.
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend(script.encode_utf16().flat_map(|u| u.to_le_bytes()));
    file.write_all(&bytes)?;
    file.flush()?;
    let sys = win::system_dir().ok_or(CleanupError::Win32(0))?;
    guard.check(path)?;
    let params = format!("/s \"{}\"", file.path().display());
    match win::run_and_wait(&sys.join("diskpart.exe"), &params, true, false)? {
        0 => Ok(()),
        code => Err(CleanupError::ExitCode(code)),
    }
}

/// Elevated `Checkpoint-Computer`. Windows allows one restore point per 24 hours by default,
/// so a failure here is common and not fatal for non-system actions.
pub(crate) fn create_restore_point(description: &str) -> Result<(), CleanupError> {
    let clean: String = description
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == ' ' || *c == '-' || *c == '_')
        .take(64)
        .collect();
    let sys = win::system_dir().ok_or(CleanupError::Win32(0))?;
    let ps = sys.join("WindowsPowerShell").join("v1.0").join("powershell.exe");
    let params = format!(
        "-NoProfile -NonInteractive -Command \"Checkpoint-Computer -Description '{clean}' -RestorePointType MODIFY_SETTINGS -ErrorAction Stop\""
    );
    match win::run_and_wait(&ps, &params, true, false)? {
        0 => Ok(()),
        code => Err(CleanupError::ExitCode(code)),
    }
}
