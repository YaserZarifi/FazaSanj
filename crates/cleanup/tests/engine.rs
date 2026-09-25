//! Engine tests. Everything happens inside a fresh temp dir. Tests that are not blocked use
//! permanent delete (safe + knowledge base + permanent_for_safe) so nothing lands in the real
//! Recycle Bin, except the one ignored recycle test.

use std::collections::HashSet;
use std::fs;
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use fazasanj_cleanup::{ActionExtras, CleanupEngine, CommandSpec};
use fazasanj_model::{
    ActionStatus, Bilingual, CleanupAction, CleanupMethod, CleanupOptions, CleanupPlan, CleanupReport, CleanupTarget,
    Explanation, ExplanationSource, PlanWarningCode, SafetyLevel,
};
use fazasanj_safety::SafetyContext;

fn explanation(method: CleanupMethod, safety: SafetyLevel, source: ExplanationSource) -> Explanation {
    Explanation {
        rule_id: "test-rule".into(),
        source,
        title: Bilingual::new("آزمایش", "test"),
        why_big: Bilingual::new("", ""),
        if_deleted: Bilingual::new("", ""),
        safety,
        method,
        needs_admin: false,
        instructions: None,
        confidence: None,
    }
}

fn target(path: &Path, method: CleanupMethod) -> CleanupTarget {
    CleanupTarget {
        path: path.to_string_lossy().into_owned(),
        bytes: 0,
        explanation: explanation(method, SafetyLevel::Safe, ExplanationSource::KnowledgeBase),
    }
}

/// Debug engine sandboxed to `root`, with `protected` as extra protected trees.
fn engine(root: &Path, protected: &[PathBuf]) -> CleanupEngine {
    let ctx = SafetyContext::new(
        protected.iter().map(|p| p.to_string_lossy().into_owned()).collect(),
        vec![],
        None,
    );
    CleanupEngine::new(ctx, Some(root.to_path_buf()), true)
}

fn opts(dry_run: bool) -> CleanupOptions {
    CleanupOptions { dry_run, permanent_for_safe: true, create_restore_point: false }
}

fn run(e: &CleanupEngine, plan: &CleanupPlan, dry: bool) -> CleanupReport {
    e.execute(plan, opts(dry), &AtomicBool::new(false), &|_| {})
}

/// A plan written by hand, as if the UI or a bug sent something the planner never made.
fn raw_plan(path: &Path, method: CleanupMethod, command: Option<&str>) -> CleanupPlan {
    CleanupPlan {
        plan_id: 1,
        actions: vec![CleanupAction {
            index: 0,
            path: path.to_string_lossy().into_owned(),
            title: Bilingual::new("x", "x"),
            method,
            bytes: 0,
            safety: SafetyLevel::Safe,
            consequence: Bilingual::new("x", "x"),
            needs_admin: false,
            permanent: true,
            instructions: None,
            command: command.map(str::to_string),
            blocked: false,
        }],
        total_bytes: 0,
        needs_admin: false,
        has_system_actions: false,
        warnings: vec![],
    }
}

fn write(p: &Path, bytes: usize) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, vec![7u8; bytes]).unwrap();
}

fn make_junk(dir: &Path) {
    write(&dir.join("a.tmp"), 1000);
    write(&dir.join("b.tmp"), 2000);
    write(&dir.join("sub").join("c.tmp"), 3000);
    write(&dir.join("sub").join("deep").join("d.tmp"), 4000);
    fs::create_dir_all(dir.join("empty")).unwrap();
}

fn count_files(dir: &Path) -> usize {
    let mut n = 0;
    for e in fs::read_dir(dir).unwrap().flatten() {
        let ft = e.file_type().unwrap();
        if ft.is_dir() && !ft.is_symlink() {
            n += count_files(&e.path());
        } else {
            n += 1;
        }
    }
    n
}

#[test]
fn delete_contents_keeps_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    make_junk(&cache);
    let e = engine(tmp.path(), &[]);
    let plan = e.build_plan(1, &[target(&cache, CleanupMethod::DeleteContents)]);
    assert!(!plan.actions[0].blocked);
    let r = run(&e, &plan, false);
    let res = &r.results[0];
    assert_eq!(res.status, ActionStatus::Done, "{res:?}");
    assert_eq!(res.bytes_freed, 10_000);
    assert_eq!(res.files_removed, 4);
    assert!(cache.is_dir());
    assert_eq!(fs::read_dir(&cache).unwrap().count(), 0);
}

#[test]
fn dry_run_matches_real_run() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    make_junk(&cache);
    write(&cache.join("sub").join("more.bin"), 12_345);
    let e = engine(tmp.path(), &[]);
    let plan = e.build_plan(1, &[target(&cache, CleanupMethod::DeleteContents)]);

    let dry = run(&e, &plan, true);
    let d = &dry.results[0];
    assert_eq!(d.status, ActionStatus::DryRun);
    assert_eq!(count_files(&cache), 5, "dry run must not remove anything");
    assert!(!d.would_remove.is_empty());

    let real = run(&e, &plan, false);
    let r = &real.results[0];
    assert_eq!(d.bytes_freed, r.bytes_freed);
    assert_eq!(d.files_removed, r.files_removed);
    assert_eq!(dry.bytes_freed, real.bytes_freed);
    assert_eq!(r.bytes_freed, 10_000 + 12_345);
    for p in &d.would_remove {
        assert!(fs::symlink_metadata(p).is_err(), "{p} should be gone");
    }
    assert!(r.would_remove.is_empty());
}

#[test]
fn dry_run_matches_real_run_with_locked_file() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    make_junk(&cache);
    let locked = cache.join("sub").join("locked.db");
    write(&locked, 500);
    let _hold = fs::OpenOptions::new().read(true).share_mode(0).open(&locked).unwrap();
    let e = engine(tmp.path(), &[]);
    let plan = e.build_plan(1, &[target(&cache, CleanupMethod::DeleteContents)]);
    let dry = run(&e, &plan, true);
    let real = run(&e, &plan, false);
    let (d, r) = (&dry.results[0], &real.results[0]);
    assert_eq!(d.bytes_freed, r.bytes_freed);
    assert_eq!(d.files_removed, r.files_removed);
    assert_eq!(d.files_skipped, r.files_skipped);
    assert_eq!(d.skipped_paths, r.skipped_paths);
}

#[test]
fn locked_file_is_skipped_not_forced() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    make_junk(&cache);
    let locked = cache.join("sub").join("locked.db");
    write(&locked, 500);
    let hold = fs::OpenOptions::new().read(true).share_mode(0).open(&locked).unwrap();
    let e = engine(tmp.path(), &[]);
    let plan = e.build_plan(1, &[target(&cache, CleanupMethod::DeleteContents)]);
    let r = run(&e, &plan, false);
    let res = &r.results[0];
    assert_eq!(res.status, ActionStatus::Partial, "{res:?}");
    assert_eq!(res.files_skipped, 1);
    assert_eq!(res.files_removed, 4);
    assert!(res.skipped_paths.iter().any(|p| p.ends_with("locked.db")));
    assert_eq!(res.error.as_ref().unwrap().code, "in_use");
    drop(hold);
    assert!(locked.exists(), "locked file must still be there");
    assert!(cache.join("sub").is_dir(), "its folder stays too");
}

#[test]
fn only_locked_file_gives_skipped_in_use() {
    let tmp = tempfile::tempdir().unwrap();
    let f = tmp.path().join("app.log");
    write(&f, 10);
    let _hold = fs::OpenOptions::new().read(true).share_mode(0).open(&f).unwrap();
    let e = engine(tmp.path(), &[]);
    let plan = e.build_plan(1, &[target(&f, CleanupMethod::Recycle)]);
    let r = run(&e, &plan, false);
    assert_eq!(r.results[0].status, ActionStatus::SkippedInUse);
    assert!(f.exists());
}

fn mklink_junction(link: &Path, target: &Path) -> bool {
    std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn junction_inside_folder_keeps_its_target() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    let precious = tmp.path().join("precious");
    make_junk(&cache);
    write(&precious.join("keep.txt"), 100);
    write(&precious.join("inner").join("keep2.txt"), 100);
    assert!(mklink_junction(&cache.join("sub").join("jump"), &precious), "mklink /J failed");

    let e = engine(tmp.path(), &[]);
    let plan = e.build_plan(1, &[target(&cache, CleanupMethod::DeleteContents)]);
    let dry = run(&e, &plan, true);
    let real = run(&e, &plan, false);
    assert_eq!(dry.results[0].bytes_freed, real.results[0].bytes_freed);
    assert_eq!(real.results[0].status, ActionStatus::Done, "{:?}", real.results[0]);
    assert_eq!(real.results[0].bytes_freed, 10_000);
    assert!(precious.join("keep.txt").exists());
    assert!(precious.join("inner").join("keep2.txt").exists());
    assert!(!cache.join("sub").exists());
}

#[test]
fn junction_as_target_removes_only_the_link() {
    let tmp = tempfile::tempdir().unwrap();
    let precious = tmp.path().join("precious");
    write(&precious.join("keep.txt"), 100);
    let link = tmp.path().join("link");
    assert!(mklink_junction(&link, &precious));
    let e = engine(tmp.path(), &[]);
    // delete_contents on a link must not walk into the target.
    let plan = e.build_plan(1, &[target(&link, CleanupMethod::DeleteContents)]);
    let r = run(&e, &plan, false);
    assert_eq!(r.results[0].status, ActionStatus::Failed);
    assert_eq!(r.results[0].error.as_ref().unwrap().code, "not_a_folder");
    assert!(precious.join("keep.txt").exists());
    // Removing it removes the link only.
    let plan = e.build_plan(1, &[target(&link, CleanupMethod::Recycle)]);
    let r = run(&e, &plan, false);
    assert_eq!(r.results[0].status, ActionStatus::Done, "{:?}", r.results[0]);
    assert!(fs::symlink_metadata(&link).is_err());
    assert!(precious.join("keep.txt").exists());
}

#[test]
fn file_symlink_inside_folder_keeps_its_target() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    make_junk(&cache);
    let precious = tmp.path().join("precious.txt");
    write(&precious, 100);
    if std::os::windows::fs::symlink_file(&precious, cache.join("link.txt")).is_err() {
        eprintln!("skipping: creating symlinks needs admin or developer mode");
        return;
    }
    let e = engine(tmp.path(), &[]);
    let plan = e.build_plan(1, &[target(&cache, CleanupMethod::DeleteContents)]);
    let r = run(&e, &plan, false);
    assert_eq!(r.results[0].status, ActionStatus::Done);
    assert!(precious.exists());
    assert_eq!(fs::read(&precious).unwrap().len(), 100);
}

#[test]
fn block_list_refuses_hand_made_plans_for_every_executor() {
    let tmp = tempfile::tempdir().unwrap();
    let protected = tmp.path().join("protected");
    make_junk(&protected);
    let vhdx = protected.join("disk.vhdx");
    write(&vhdx, 100);
    let e = engine(tmp.path(), std::slice::from_ref(&protected));

    let cases = [
        (protected.clone(), CleanupMethod::Recycle, None),
        (protected.clone(), CleanupMethod::DeleteContents, None),
        (protected.join("sub"), CleanupMethod::DeleteContents, None),
        (protected.join("a.tmp"), CleanupMethod::Recycle, None),
        (vhdx.clone(), CleanupMethod::CompactVhdx, None),
        // The parent of a protected tree is protected too.
        (tmp.path().to_path_buf(), CleanupMethod::DeleteContents, None),
    ];
    for (path, method, cmd) in cases {
        let plan = raw_plan(&path, method, cmd);
        for dry in [true, false] {
            let r = run(&e, &plan, dry);
            let res = &r.results[0];
            assert_eq!(res.status, ActionStatus::Blocked, "{method:?} {path:?} {res:?}");
            assert!(res.error.as_ref().unwrap().code.starts_with("blocked"), "{res:?}");
            assert_eq!(res.bytes_freed, 0);
        }
        // The planner marks them too.
        let planned = e.build_plan(1, &[target(&path, method)]);
        assert!(planned.actions[0].blocked, "{path:?}");
        assert!(planned.warnings.iter().any(|w| w.code == PlanWarningCode::Blocked));
    }
    assert_eq!(count_files(&protected), 5);
}

#[test]
fn block_list_is_checked_for_every_child() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    make_junk(&cache);
    // Registry hive names are blocked wherever they are.
    let hive = cache.join("sub").join("NTUSER.DAT");
    write(&hive, 50);
    let e = engine(tmp.path(), &[]);
    let plan = raw_plan(&cache, CleanupMethod::DeleteContents, None);
    let r = run(&e, &plan, false);
    let res = &r.results[0];
    assert_eq!(res.status, ActionStatus::Partial, "{res:?}");
    assert_eq!(res.error.as_ref().unwrap().code, "blocked_registry_hive");
    assert!(hive.exists());
    assert_eq!(res.files_removed, 4);
}

#[test]
fn plan_marked_blocked_is_never_run() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    make_junk(&cache);
    let e = engine(tmp.path(), &[]);
    let mut plan = raw_plan(&cache, CleanupMethod::DeleteContents, None);
    plan.actions[0].blocked = true;
    let r = run(&e, &plan, false);
    assert_eq!(r.results[0].status, ActionStatus::Blocked);
    assert_eq!(count_files(&cache), 4);
}

#[test]
fn sandbox_refuses_outside_paths_in_debug() {
    let tmp = tempfile::tempdir().unwrap();
    let sandbox = tmp.path().join("sandbox");
    let outside = tmp.path().join("outside");
    fs::create_dir_all(&sandbox).unwrap();
    make_junk(&outside);
    let ctx = SafetyContext::new(vec![], vec![], None);
    let e = CleanupEngine::new(ctx.clone(), Some(sandbox.clone()), true);
    for method in [CleanupMethod::Recycle, CleanupMethod::DeleteContents, CleanupMethod::CompactVhdx] {
        let plan = raw_plan(&outside, method, None);
        let res = &run(&e, &plan, false).results[0];
        assert_eq!(res.status, ActionStatus::Blocked);
        assert_eq!(res.error.as_ref().unwrap().code, "outside_sandbox");
        let planned = e.build_plan(1, &[target(&outside, method)]);
        assert!(planned.actions[0].blocked);
        assert!(planned.warnings.iter().any(|w| w.code == PlanWarningCode::OutsideSandbox));
    }
    // Commands are checked against the sandbox as well (never actually started here).
    let plan = raw_plan(Path::new(r"C:\Windows\WinSxS"), CleanupMethod::Command, Some("dism.exe /Online /Cleanup-Image /StartComponentCleanup"));
    let res = &run(&e, &plan, false).results[0];
    assert_eq!(res.status, ActionStatus::Blocked);
    assert_eq!(res.error.as_ref().unwrap().code, "outside_sandbox");

    let no_sb = CleanupEngine::new(ctx, None, true);
    let plan = raw_plan(&outside, CleanupMethod::DeleteContents, None);
    let res = &run(&no_sb, &plan, false).results[0];
    assert_eq!(res.status, ActionStatus::Blocked);
    assert_eq!(res.error.as_ref().unwrap().code, "sandbox_not_configured");
    assert_eq!(count_files(&outside), 4);
}

#[test]
fn commands_outside_the_allow_list_are_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let e = engine(tmp.path(), &[]);
    for bad in ["cmd.exe /c rd /s /q C:\\", "dism.exe /Online /Cleanup-Image /StartComponentCleanup /ResetBase"] {
        let plan = raw_plan(&tmp.path().join("x"), CleanupMethod::Command, Some(bad));
        let res = &run(&e, &plan, false).results[0];
        assert_eq!(res.status, ActionStatus::Failed);
        assert_eq!(res.error.as_ref().unwrap().code, "command_not_allowed");
    }
    let plan = raw_plan(&tmp.path().join("x"), CleanupMethod::OpenAppSetting, Some("calc.exe"));
    let res = &run(&e, &plan, false).results[0];
    assert_eq!(res.error.as_ref().unwrap().code, "open_target_not_allowed");
    // Allowed ones in a dry run do nothing.
    let plan = raw_plan(&tmp.path().join("x"), CleanupMethod::Command, Some("powercfg.exe /hibernate off"));
    assert_eq!(run(&e, &plan, true).results[0].status, ActionStatus::DryRun);
}

#[test]
fn vhdx_in_use_is_skipped() {
    let tmp = tempfile::tempdir().unwrap();
    let disk = tmp.path().join("ext4.vhdx");
    write(&disk, 4096);
    let _hold = fs::OpenOptions::new().read(true).share_mode(0).open(&disk).unwrap();
    let e = engine(tmp.path(), &[]);
    let plan = e.build_plan(1, &[target(&disk, CleanupMethod::CompactVhdx)]);
    assert!(plan.has_system_actions);
    // Dry run still checks the lock, and never starts diskpart.
    let res = &run(&e, &plan, true).results[0];
    assert_eq!(res.status, ActionStatus::SkippedInUse);
    assert_eq!(res.error.as_ref().unwrap().code, "vhdx_in_use");
    assert_eq!(fs::metadata(&disk).unwrap().len(), 4096);
}

#[test]
fn compact_refuses_non_disk_files() {
    let tmp = tempfile::tempdir().unwrap();
    let f = tmp.path().join("notes.txt");
    write(&f, 10);
    let e = engine(tmp.path(), &[]);
    let plan = e.build_plan(1, &[target(&f, CleanupMethod::CompactVhdx)]);
    let res = &run(&e, &plan, true).results[0];
    assert_eq!(res.error.as_ref().unwrap().code, "not_vhdx");
    assert!(f.exists());
}

#[test]
fn manual_and_settings_touch_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let f = tmp.path().join("big.iso");
    write(&f, 10);
    let e = engine(tmp.path(), &[]);
    let plan = e.build_plan(1, &[target(&f, CleanupMethod::ManualOnly), target(&f, CleanupMethod::OpenAppSetting)]);
    let r = run(&e, &plan, false);
    assert_eq!(r.results[0].status, ActionStatus::NeedsManual);
    // No open target given, so it falls back to the written instructions.
    assert_eq!(r.results[1].status, ActionStatus::NeedsManual);
    assert!(f.exists());
}

#[test]
fn plan_fields_and_warnings() {
    let tmp = tempfile::tempdir().unwrap();
    let e = CleanupEngine::new(SafetyContext::new(vec![], vec![], None), None, false);
    let cache = tmp.path().join("cache");
    let mut t = target(&cache, CleanupMethod::Recycle);
    t.bytes = 500;
    let mut dnt = target(&tmp.path().join("keep"), CleanupMethod::DeleteContents);
    dnt.explanation.safety = SafetyLevel::DoNotTouch;
    dnt.bytes = 1000;
    let sys = CleanupTarget {
        path: r"C:\hiberfil.sys".into(),
        bytes: 8_000,
        explanation: explanation(CleanupMethod::Command, SafetyLevel::Careful, ExplanationSource::KnowledgeBase),
    };
    let setting = CleanupTarget {
        path: r"C:\System Volume Information".into(),
        bytes: 1,
        explanation: explanation(CleanupMethod::OpenAppSetting, SafetyLevel::Careful, ExplanationSource::KnowledgeBase),
    };
    let extras = |t: &CleanupTarget| {
        if t.explanation.method == CleanupMethod::OpenAppSetting {
            ActionExtras { open_target: Some("SystemPropertiesProtection.exe".into()), ..Default::default() }
        } else if t.explanation.method == CleanupMethod::Command {
            ActionExtras { command: Some(CommandSpec::new("powercfg", &["/h", "off"])), ..Default::default() }
        } else {
            ActionExtras::default()
        }
    };
    let plan = e.build_plan_with(9, &[t, dnt, sys, setting], &extras);
    assert_eq!(plan.plan_id, 9);
    assert!(!plan.actions[0].blocked);
    assert!(!plan.actions[0].permanent);
    assert!(plan.actions[1].blocked, "do_not_touch with a destructive method");
    assert_eq!(plan.actions[2].command.as_deref(), Some("powercfg.exe /hibernate off"));
    assert!(plan.actions[2].needs_admin);
    assert_eq!(plan.actions[3].command.as_deref(), Some("SystemPropertiesProtection.exe"));
    assert_eq!(plan.total_bytes, 500 + 8_000 + 1);
    assert!(plan.needs_admin);
    assert!(plan.has_system_actions);
    let has = |code, i: usize| {
        let p = &plan.actions[i].path;
        plan.warnings.iter().any(|w| w.code == code && w.path.as_ref() == Some(p))
    };
    assert!(has(PlanWarningCode::RecycleSameDrive, 0));
    assert!(has(PlanWarningCode::Blocked, 1));
    assert!(has(PlanWarningCode::SystemAction, 2));
    assert!(has(PlanWarningCode::NeedsAdmin, 2));
}

#[test]
fn heuristic_and_ai_items_are_never_safe_or_system() {
    let tmp = tempfile::tempdir().unwrap();
    let e = engine(tmp.path(), &[]);
    let mut targets = Vec::new();
    for source in [ExplanationSource::Heuristic, ExplanationSource::Ai] {
        for method in [CleanupMethod::Recycle, CleanupMethod::Command, CleanupMethod::CompactVhdx] {
            targets.push(CleanupTarget {
                path: tmp.path().join("x.vhdx").to_string_lossy().into_owned(),
                bytes: 1,
                explanation: explanation(method, SafetyLevel::Safe, source),
            });
        }
    }
    let plan = e.build_plan(1, &targets);
    for a in &plan.actions {
        assert_ne!(a.safety, SafetyLevel::Safe);
        assert!(!a.permanent);
        if a.method != CleanupMethod::Recycle {
            assert!(a.blocked, "{:?}", a.method);
        }
    }
}

#[test]
fn cancel_before_start_touches_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    make_junk(&cache);
    let e = engine(tmp.path(), &[]);
    let plan = e.build_plan(1, &[target(&cache, CleanupMethod::DeleteContents)]);
    let r = e.execute(&plan, opts(false), &AtomicBool::new(true), &|_| {});
    assert_eq!(r.results[0].error.as_ref().unwrap().code, "cancelled");
    assert_eq!(count_files(&cache), 4);
}

#[test]
fn progress_reports_and_free_space() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    make_junk(&cache);
    let e = engine(tmp.path(), &[]);
    let plan = e.build_plan(1, &[target(&cache, CleanupMethod::DeleteContents)]);
    let seen = std::cell::RefCell::new(Vec::new());
    let r = e.execute(&plan, opts(false), &AtomicBool::new(false), &|p| seen.borrow_mut().push(p));
    assert!(!seen.borrow().is_empty());
    assert!(r.free_before > 0 && r.free_after > 0);
    assert!(r.finished_at >= r.started_at);
    assert_eq!(r.restore_point_created, None);
}

#[test]
fn read_only_files_are_removed() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    let f = cache.join("ro.pack");
    write(&f, 100);
    let mut perm = fs::metadata(&f).unwrap().permissions();
    perm.set_readonly(true);
    fs::set_permissions(&f, perm).unwrap();
    let e = engine(tmp.path(), &[]);
    let plan = e.build_plan(1, &[target(&cache, CleanupMethod::DeleteContents)]);
    let r = run(&e, &plan, false);
    assert_eq!(r.results[0].status, ActionStatus::Done);
    assert!(!f.exists());
}

/// Puts two small files in the real Recycle Bin, so it only runs on request:
/// `cargo test -p fazasanj-cleanup -- --ignored`
#[test]
#[ignore]
fn recycle_goes_to_the_bin_and_can_be_found() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    write(&cache.join("one.tmp"), 10);
    write(&cache.join("two.tmp"), 20);
    let e = engine(tmp.path(), &[]);
    let mut t = target(&cache, CleanupMethod::DeleteContents);
    t.explanation.safety = SafetyLevel::ProbablySafe;
    let plan = e.build_plan(1, &[t]);
    let r = run(&e, &plan, false);
    assert_eq!(r.results[0].status, ActionStatus::Done, "{:?}", r.results[0]);
    assert_eq!(r.results[0].bytes_freed, 30);
    let found: HashSet<String> = fazasanj_cleanup::recycle_bin_lookup(&[cache.to_string_lossy().into_owned()]);
    assert_eq!(found.len(), 1);
}
