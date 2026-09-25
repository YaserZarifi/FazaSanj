//! Removes (or, in a dry run, pretends to remove) a scanned tree. Both modes walk the same
//! code, only the destructive call is skipped, so their numbers match.

use std::cell::Cell;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::error::CleanupError;
use crate::guard::Guard;
use crate::walk::{io_problem, Entry, Kind};
use crate::win;

/// How many paths we keep for `would_remove` and `skipped_paths`.
pub(crate) const PATH_CAP: usize = 200;
const RECYCLE_BATCH: usize = 64;

#[derive(Debug, Default)]
pub(crate) struct Tally {
    pub files_removed: u64,
    pub bytes: u64,
    pub files_skipped: u64,
    pub removed_paths: Vec<String>,
    pub skipped_paths: Vec<String>,
    /// First reason something was left behind.
    pub first_problem: Option<CleanupError>,
    pub cancelled: bool,
}

impl Tally {
    fn removed(&mut self, path: &Path, files: u64, bytes: u64) {
        self.files_removed += files;
        self.bytes += bytes;
        if self.removed_paths.len() < PATH_CAP {
            self.removed_paths.push(path.to_string_lossy().into_owned());
        }
    }

    fn skipped(&mut self, path: &Path, files: u64, why: CleanupError) {
        self.files_skipped += files.max(1);
        if self.skipped_paths.len() < PATH_CAP {
            self.skipped_paths.push(path.to_string_lossy().into_owned());
        }
        if self.first_problem.is_none() {
            self.first_problem = Some(why);
        }
    }
}

pub(crate) struct Ctx<'a> {
    pub guard: &'a Guard,
    pub dry: bool,
    pub cancel: &'a AtomicBool,
    /// Called with the current path and bytes removed so far in this action.
    pub progress: &'a dyn Fn(&Path, u64),
    last: Cell<Option<Instant>>,
}

impl<'a> Ctx<'a> {
    pub fn new(guard: &'a Guard, dry: bool, cancel: &'a AtomicBool, progress: &'a dyn Fn(&Path, u64)) -> Self {
        Self { guard, dry, cancel, progress, last: Cell::new(None) }
    }

    fn tick(&self, path: &Path, bytes: u64) {
        let now = Instant::now();
        let due = match self.last.get() {
            Some(t) => now.duration_since(t) >= Duration::from_millis(100),
            None => true,
        };
        if due {
            self.last.set(Some(now));
            (self.progress)(path, bytes);
        }
    }

    fn cancelled(&self, tally: &mut Tally) -> bool {
        if self.cancel.load(Ordering::Relaxed) {
            tally.cancelled = true;
        }
        tally.cancelled
    }
}

/// Deletes `entry` bottom-up without the Recycle Bin. With `remove_self` false only the
/// contents go and the folder stays. Returns true when the entry is gone.
pub(crate) fn delete_permanently(entry: &Entry, remove_self: bool, ctx: &Ctx, tally: &mut Tally) -> bool {
    if ctx.cancelled(tally) {
        return false;
    }
    if let Some(p) = &entry.problem {
        tally.skipped(&entry.path, entry.total_files, p.clone());
        return false;
    }
    match entry.kind {
        Kind::Dir => {
            let mut all = true;
            for c in &entry.children {
                all &= delete_permanently(c, true, ctx, tally);
            }
            if !remove_self || !all {
                return false;
            }
            // The files inside are already counted, the folder itself adds nothing.
            remove_one(entry, ctx, tally, 0, 0)
        }
        Kind::File => remove_one(entry, ctx, tally, 1, entry.size),
        Kind::Link { .. } => remove_one(entry, ctx, tally, 1, 0),
    }
}

fn remove_one(entry: &Entry, ctx: &Ctx, tally: &mut Tally, files: u64, bytes: u64) -> bool {
    // Checked again right before the call, the tree may be minutes old.
    if let Err(e) = ctx.guard.check(&entry.path) {
        tally.skipped(&entry.path, files, e);
        return false;
    }
    if !ctx.dry {
        if let Err(e) = remove_path(entry) {
            tally.skipped(&entry.path, files, e);
            return false;
        }
    }
    tally.removed(&entry.path, files, bytes);
    ctx.tick(&entry.path, tally.bytes);
    true
}

fn remove_path(entry: &Entry) -> Result<(), CleanupError> {
    let p = &entry.path;
    let r = match entry.kind {
        // RemoveDirectoryW on a junction or directory symlink removes the link, not the target.
        Kind::Dir | Kind::Link { dir: true } => std::fs::remove_dir(p),
        Kind::File | Kind::Link { dir: false } => {
            if entry.readonly && entry.kind == Kind::File {
                remove_readonly_file(p)
            } else {
                std::fs::remove_file(p)
            }
        }
    };
    r.map_err(|e| io_problem(&e))
}

/// Read-only cache files are common (git objects, some installers). Clearing the flag is not
/// forcing anything, but put it back if the delete still fails.
fn remove_readonly_file(p: &Path) -> std::io::Result<()> {
    let meta = std::fs::symlink_metadata(p)?;
    let mut perm = meta.permissions();
    // Windows only crate, this just clears FILE_ATTRIBUTE_READONLY.
    #[allow(clippy::permissions_set_readonly_false)]
    perm.set_readonly(false);
    std::fs::set_permissions(p, perm.clone())?;
    let r = std::fs::remove_file(p);
    if r.is_err() {
        perm.set_readonly(true);
        let _ = std::fs::set_permissions(p, perm);
    }
    r
}

/// Sends `entry` to the Recycle Bin. If something inside is in use or protected, it goes one
/// level down and recycles what it can. Links are removed as links (they hold no data).
pub(crate) fn recycle(entry: &Entry, ctx: &Ctx, tally: &mut Tally) {
    let mut whole: Vec<&Entry> = Vec::new();
    collect_recyclable(entry, ctx, tally, &mut whole);
    for chunk in whole.chunks(RECYCLE_BATCH) {
        if ctx.cancelled(tally) {
            return;
        }
        let mut ready: Vec<&Entry> = Vec::with_capacity(chunk.len());
        for e in chunk {
            match ctx.guard.check(&e.path) {
                Ok(()) => ready.push(e),
                Err(err) => tally.skipped(&e.path, e.total_files, err),
            }
        }
        if ctx.dry {
            for e in ready {
                tally.removed(&e.path, e.total_files, e.total_bytes);
            }
            continue;
        }
        let paths: Vec<&Path> = ready.iter().map(|e| e.path.as_path()).collect();
        let call = win::recycle(&paths);
        // The shell can stop halfway, so what is gone is what counts.
        for e in ready {
            if std::fs::symlink_metadata(&e.path).is_err() {
                tally.removed(&e.path, e.total_files, e.total_bytes);
            } else {
                let why = call.clone().err().unwrap_or(CleanupError::InUse);
                tally.skipped(&e.path, e.total_files, why);
            }
        }
        if let Some(last) = chunk.last() {
            ctx.tick(&last.path, tally.bytes);
        }
    }
}

fn collect_recyclable<'e>(entry: &'e Entry, ctx: &Ctx, tally: &mut Tally, out: &mut Vec<&'e Entry>) {
    if ctx.cancelled(tally) {
        return;
    }
    if let Some(p) = &entry.problem {
        tally.skipped(&entry.path, entry.total_files, p.clone());
        return;
    }
    if entry.whole {
        out.push(entry);
        return;
    }
    match entry.kind {
        Kind::Link { .. } => {
            remove_one(entry, ctx, tally, 1, 0);
        }
        Kind::Dir => {
            for c in &entry.children {
                collect_recyclable(c, ctx, tally, out);
            }
        }
        // A file without a problem is always whole.
        Kind::File => out.push(entry),
    }
}
