use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use fazasanj_rules::RuleSet;
use junk_gen::{run, Guard, JunkError, Kind, Options, Scale, MARKER};

/// Temp folders live on the system drive here, so tests use a guard that refuses another letter.
fn test_guard() -> Guard {
    Guard::refusing(&["Q:"])
}

fn opts(target: &Path, dry_run: bool, force: bool) -> Options {
    Options { target: target.to_path_buf(), scale: Scale::Tiny, seed: 7, dry_run, force }
}

fn ms(t: SystemTime) -> i64 {
    t.duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

#[test]
fn cli_refuses_c_and_system_drive() {
    let exe = env!("CARGO_BIN_EXE_junk-gen");
    let sd = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
    let probe = format!(r"{sd}\fazasanj-junk-gen-must-not-exist");
    for target in [r"C:\".to_string(), r"C:\fazasanj-junk-gen-must-not-exist".to_string(), probe.clone()] {
        let out = Command::new(exe).arg(&target).arg("--scale").arg("small").output().unwrap();
        assert!(!out.status.success(), "{target} was accepted");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.contains("system drive"), "{err}");
    }
    assert!(!Path::new(&probe).exists());
    assert!(!Path::new(r"C:\fazasanj-junk-gen-must-not-exist").exists());
}

#[test]
fn cli_rejects_bad_args() {
    let exe = env!("CARGO_BIN_EXE_junk-gen");
    let out = Command::new(exe).arg("--scale").arg("huge").output().unwrap();
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn dry_run_writes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("junk");
    let (plan, summary) = run(&opts(&target, true, false), &test_guard()).unwrap();
    assert!(!target.exists());
    assert!(!plan.entries.is_empty());
    assert!(summary.bytes > 0 && summary.dry_run);
    assert_eq!(std::fs::read_dir(tmp.path()).unwrap().count(), 0);
}

#[test]
fn refuses_foreign_non_empty_folder_and_real_windows() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("important.txt"), "x").unwrap();
    let r = run(&opts(tmp.path(), false, true), &test_guard());
    assert!(matches!(r, Err(JunkError::NotEmpty(_))), "{r:?}");

    let win = tempfile::tempdir().unwrap();
    let sys = win.path().join("Windows").join("System32");
    std::fs::create_dir_all(&sys).unwrap();
    std::fs::write(sys.join("ntoskrnl.exe"), "x").unwrap();
    let r = run(&opts(win.path(), false, true), &test_guard());
    assert!(matches!(r, Err(JunkError::RealWindows(_))), "{r:?}");
}

#[test]
fn plan_covers_every_rule() {
    let rules = RuleSet::load_embedded().unwrap();
    let plan = junk_gen::plan::build(&rules, Scale::Small, 1);
    let summary = junk_gen::summarize(&plan, &rules, true);
    assert_eq!(summary.rules_covered, rules.len());
    let total = plan.total_bytes() as f64;
    let want = Scale::Small.budget() as f64;
    assert!(total > want * 0.6 && total < want * 1.4, "{total} vs {want}");
    // Same seed, same plan.
    let again = junk_gen::plan::build(&rules, Scale::Small, 1);
    assert_eq!(plan.entries, again.entries);
}

/// Walks the generated tree top-down the way the app does, with the temp folder standing in
/// for a drive root, and returns the rule the app would attach to each path.
fn walk(rules: &RuleSet, root: &Path) -> HashMap<String, String> {
    let now = ms(SystemTime::now());
    let mut found = HashMap::new();
    let mut stack = vec![(root.to_path_buf(), "T:".to_string())];
    while let Some((dir, tpath)) = stack.pop() {
        for e in std::fs::read_dir(&dir).unwrap() {
            let e = e.unwrap();
            let meta = e.metadata().unwrap();
            let name = e.file_name().to_string_lossy().into_owned();
            let child = format!(r"{tpath}\{name}");
            let modified = meta.modified().ok().map(ms);
            let hit = rules.match_path(&child, meta.is_dir(), modified, now);
            if let Some(i) = hit {
                found.insert(child.to_ascii_lowercase(), rules.get(i).unwrap().id.clone());
            }
            let stop = hit.and_then(|i| rules.get(i)).is_some_and(|r| r.stops_descent());
            if meta.is_dir() && !stop {
                stack.push((e.path(), child));
            }
        }
    }
    found
}

#[test]
fn generated_tree_is_recognized() {
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("t");
    let (plan, summary) = run(&opts(&target, false, false), &test_guard()).unwrap();
    assert!(target.join(MARKER).is_file());
    assert_eq!(summary.rules_covered, summary.rules_total);

    let rules = RuleSet::load_embedded().unwrap();
    let found = walk(&rules, &target);
    for e in &plan.entries {
        let Some(id) = &e.rule_id else { continue };
        let key = format!(r"T:\{}", e.rel).to_ascii_lowercase();
        assert_eq!(found.get(&key), Some(id), "{} not recognized", e.rel);
    }

    // The fresh installer is not old enough for the rule.
    assert!(!found.contains_key(r"t:\users\test\downloads\new-setup.exe"));
}

#[test]
fn old_files_and_duplicates_look_right() {
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("t");
    let (plan, _) = run(&opts(&target, false, false), &test_guard()).unwrap();
    let now = SystemTime::now();

    let installer = plan.entries.iter().find(|e| e.rule_id.as_deref() == Some("old-installers")).unwrap();
    let m = std::fs::metadata(target.join(&installer.rel)).unwrap().modified().unwrap();
    assert!(now.duration_since(m).unwrap().as_secs() > 100 * 86_400);

    let pic = std::fs::read(target.join(r"Users\Test\Pictures\Trip\IMG_0001.jpg")).unwrap();
    let copy = std::fs::read(target.join(r"Users\Test\Desktop\backup\IMG_0001.jpg")).unwrap();
    assert_eq!(pic, copy);
    let a = std::fs::read(target.join(r"Users\Test\Pictures\Trip\IMG_0010.jpg")).unwrap();
    let b = std::fs::read(target.join(r"Users\Test\Desktop\backup\IMG_0010.jpg")).unwrap();
    assert_eq!(a.len(), b.len());
    assert_eq!(a.iter().zip(&b).filter(|(x, y)| x != y).count(), 1);

    for e in &plan.entries {
        if let Kind::File { size, .. } = e.kind {
            assert_eq!(std::fs::metadata(target.join(&e.rel)).unwrap().len(), size, "{}", e.rel);
        }
    }
}

#[test]
fn second_run_needs_force() {
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("t");
    run(&opts(&target, false, false), &test_guard()).unwrap();
    let r = run(&opts(&target, false, false), &test_guard());
    assert!(matches!(r, Err(JunkError::NeedsForce(_))), "{r:?}");
    assert!(run(&opts(&target, false, true), &test_guard()).is_ok());
}
