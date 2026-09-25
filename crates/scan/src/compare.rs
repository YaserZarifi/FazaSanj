//! Runs both scanners on the same folder and lists where they disagree.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use fazasanj_model::{PathDiff, ScanMode, ScannerComparison};

use crate::options::{ScanError, ScanOptions};
use crate::run::run_scan;
use crate::tree::ScanTree;

/// Folders are compared down to this depth below the root.
const COMPARE_DEPTH: u32 = 4;
/// Differences smaller than this are noise (files changing while we scan).
const MIN_DIFF_BYTES: u64 = 1 << 20;
const MAX_DIFFS: usize = 200;

/// Fast scan first, then the normal scan. Fails with `FastUnavailable` if fast mode fell back.
pub fn compare_scanners(
    root: &Path,
    helper_path: Option<PathBuf>,
    cancel: Arc<AtomicBool>,
) -> Result<ScannerComparison, ScanError> {
    let mut opts = ScanOptions::new(root, ScanMode::Fast);
    opts.helper_path = helper_path;
    opts.cancel = cancel;
    let fast = run_scan(opts.clone())?;
    if let Some(reason) = fast.fallback_reason {
        return Err(ScanError::FastUnavailable(reason));
    }
    opts.mode = ScanMode::Normal;
    let normal = run_scan(opts)?;
    let mut cmp = diff_trees(&normal.tree, &fast.tree);
    cmp.normal_ms = normal.duration_ms;
    cmp.fast_ms = fast.duration_ms;
    Ok(cmp)
}

/// Folder level differences of more than 1% (and at least 1 MB), biggest first.
pub fn diff_trees(normal: &ScanTree, fast: &ScanTree) -> ScannerComparison {
    let fast_sizes: HashMap<String, u64> =
        fast.folder_sizes(COMPARE_DEPTH).into_iter().map(|(p, s, _)| (p.to_lowercase(), s)).collect();
    let mut seen: HashMap<String, ()> = HashMap::new();
    let mut differences = Vec::new();
    let mut check = |path: String, n: u64, f: u64| {
        let diff = n.abs_diff(f);
        if diff >= MIN_DIFF_BYTES && diff * 100 > n.max(f) {
            differences.push(PathDiff { path, normal_bytes: n, fast_bytes: f });
        }
    };
    for (path, n, _) in normal.folder_sizes(COMPARE_DEPTH) {
        let key = path.to_lowercase();
        let f = fast_sizes.get(&key).copied().unwrap_or(0);
        seen.insert(key, ());
        check(path, n, f);
    }
    for (path, f, _) in fast.folder_sizes(COMPARE_DEPTH) {
        if !seen.contains_key(&path.to_lowercase()) {
            check(path, 0, f);
        }
    }
    differences.sort_by_key(|d| std::cmp::Reverse(d.normal_bytes.abs_diff(d.fast_bytes)));
    differences.truncate(MAX_DIFFS);
    let (ns, fs) = (normal.summary(), fast.summary());
    ScannerComparison {
        normal_bytes: ns.total_bytes,
        fast_bytes: fs.total_bytes,
        normal_files: ns.files,
        fast_files: fs.files,
        normal_ms: 0,
        fast_ms: 0,
        differences,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::TreeBuilder;
    use crate::finalize::finalize;
    use crate::node::flags::DIR;

    fn tree(a: u64, b: u64) -> ScanTree {
        let mut t = TreeBuilder::new();
        let da = t.push(0, "A", 0, 0, DIR);
        t.push(da, "f", a, 0, 0);
        let db = t.push(0, "B", 0, 0, DIR);
        t.push(db, "f", b, 0, 0);
        finalize(t, 0, r"C:\".into())
    }

    #[test]
    fn lists_only_real_differences() {
        let normal = tree(100 << 20, 50 << 20);
        let fast = tree(100 << 20, 60 << 20);
        let c = diff_trees(&normal, &fast);
        let paths: Vec<&str> = c.differences.iter().map(|d| d.path.as_str()).collect();
        assert_eq!(paths, vec![r"C:\", r"C:\B"]);
        assert_eq!(c.fast_bytes - c.normal_bytes, 10 << 20);
    }
}
