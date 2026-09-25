//! Normal walker on a real temp folder: totals, hard links, junction loops, long paths,
//! exclusions, cancel and progress.

use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use fazasanj_model::{ChildSort, ScanMode};
use fazasanj_scan::{run_scan, ScanError, ScanOptions};

fn write(path: &Path, len: usize) {
    std::fs::write(path, vec![0x5Au8; len]).expect("write file");
}

fn alloc(path: &Path) -> u64 {
    fazasanj_platform::standard_info(path).expect("standard info").allocation_size
}

/// Creates a junction without admin rights. Returns false if the system refuses.
fn junction(link: &Path, target: &Path) -> bool {
    Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn scans_temp_tree_with_links() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let sub = root.join("sub");
    std::fs::create_dir(&sub).expect("mkdir");
    let a = root.join("a.bin");
    write(&a, 100_000);
    std::fs::hard_link(&a, sub.join("a-link.bin")).expect("hard link");
    write(&sub.join("c.txt"), 5_000);
    write(&root.join("empty.txt"), 0);

    // A junction back to the root would loop forever if followed.
    let has_junction = junction(&root.join("loop"), root);
    // Directory symlinks need developer mode or admin; skip quietly when not allowed.
    let has_symlink = std::os::windows::fs::symlink_dir(&sub, root.join("sym")).is_ok();

    // Deeper than MAX_PATH.
    let mut deep = root.join("deep");
    for i in 0..30 {
        deep = deep.join(format!("level-{i:02}-xxxxxxxx"));
    }
    std::fs::create_dir_all(fazasanj_platform::to_long_path(&deep)).expect("deep dirs");
    let deep_file = deep.join("deep.bin");
    std::fs::write(fazasanj_platform::to_long_path(&deep_file), vec![1u8; 20_000]).expect("deep file");
    assert!(deep_file.to_string_lossy().len() > 260);

    let excluded = root.join("skipme");
    std::fs::create_dir(&excluded).expect("mkdir");
    write(&excluded.join("big.bin"), 1_000_000);

    let calls = Arc::new(AtomicU32::new(0));
    let mut opts = ScanOptions::new(root, ScanMode::Normal);
    opts.excluded = vec![excluded.clone()];
    let c = calls.clone();
    opts.progress = Arc::new(move |p| {
        assert_eq!(p.scanner, ScanMode::Normal);
        c.fetch_add(1, Ordering::Relaxed);
    });
    let out = run_scan(opts).expect("scan");
    let t = &out.tree;
    let s = t.summary();

    let expected = alloc(&a) + alloc(&sub.join("c.txt")) + alloc(&fazasanj_platform::to_long_path(&deep_file));
    assert_eq!(s.total_bytes, expected, "hard link counted once, excluded folder skipped");
    assert_eq!(s.hardlink_dups, 1);
    assert_eq!(out.scanner_used, ScanMode::Normal);
    assert!(out.fallback_reason.is_none());
    assert!(out.access_denied.is_empty());
    assert!(calls.load(Ordering::Relaxed) >= 1, "final progress snapshot");

    let root_display = fazasanj_scan::display_path(root);
    assert_eq!(t.root_path(), root_display);
    assert!(t.find_by_path(&excluded.to_string_lossy()).is_none());
    let deep_id = t.find_by_path(&deep_file.to_string_lossy()).expect("deep file in tree");
    assert_eq!(t.path_of(deep_id), deep_file.to_string_lossy());

    if has_junction {
        let j = t.find_by_path(&root.join("loop").to_string_lossy()).expect("junction node");
        let info = t.node_info(j).expect("info");
        assert!(info.flags.reparse);
        assert_eq!(info.child_count, 0, "junction not followed");
    }
    if has_symlink {
        let j = t.find_by_path(&root.join("sym").to_string_lossy()).expect("symlink node");
        assert_eq!(t.node_info(j).expect("info").child_count, 0);
    }

    let page = t.children_page(t.root(), ChildSort::Size, 0, 100).expect("page");
    assert_eq!(page.items[0].name, "a.bin");
}

#[test]
fn cancel_stops_the_scan() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(&dir.path().join("x.bin"), 10);
    let opts = ScanOptions::new(dir.path(), ScanMode::Normal);
    opts.cancel.store(true, Ordering::Relaxed);
    assert!(matches!(run_scan(opts), Err(ScanError::Cancelled)));
}

#[test]
fn cancel_mid_scan_of_windows_folder() {
    let cancel = Arc::new(AtomicBool::new(false));
    let mut opts = ScanOptions::new(r"C:\Windows", ScanMode::Normal);
    opts.cancel = cancel.clone();
    let c = cancel.clone();
    // Cancel on the first progress tick (100 ms in).
    opts.progress = Arc::new(move |_| c.store(true, Ordering::Relaxed));
    let started = std::time::Instant::now();
    let r = run_scan(opts);
    assert!(matches!(r, Err(ScanError::Cancelled)) || r.is_ok());
    assert!(started.elapsed().as_secs() < 20);
}

#[test]
fn missing_root_is_an_error() {
    let r = run_scan(ScanOptions::new(r"C:\surely\missing\folder", ScanMode::Normal));
    assert!(matches!(r, Err(ScanError::RootNotFound(_))));
}

#[test]
fn fast_mode_on_a_folder_falls_back_without_helper() {
    // No helper next to the test exe with this name, so fast mode must fall back cleanly
    // before any UAC prompt.
    let dir = tempfile::tempdir().expect("tempdir");
    write(&dir.path().join("x.bin"), 4096);
    let mut opts = ScanOptions::new(dir.path(), ScanMode::Fast);
    opts.helper_path = Some(dir.path().join("missing-helper.exe"));
    let fs = fazasanj_platform::filesystem_name(dir.path()).expect("fs");
    let out = run_scan(opts);
    if fs.eq_ignore_ascii_case("NTFS") {
        // The helper path does not exist: ShellExecute fails with file not found, no prompt.
        let out = out.expect("fallback scan");
        assert_eq!(out.scanner_used, ScanMode::Normal);
        assert!(out.fallback_reason.is_some());
        assert_eq!(out.tree.summary().files, 1);
    }
}
