//! Heuristics jobs: turns a scan tree into the inputs each heuristic wants.

use std::collections::HashSet;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use fazasanj_heuristics::{DirRecord, FileRecord};
use fazasanj_model::{
    ApiError, HeuristicFinding, HeuristicKind, HeuristicsDone, HeuristicsProgress, JobId, ScanId,
};
use fazasanj_scan::{ScanTree, Visit};

use crate::{now_ms, ApiResult, App, Event};

/// Duplicates smaller than this are not worth the hashing.
const DUP_MIN_BYTES: u64 = 1024 * 1024;
/// Stale files at least this big are reported one by one.
const STALE_MIN_BYTES: u64 = 100 * 1024 * 1024;

const APPDATA_CONTAINERS: [&str; 3] = ["\\appdata\\roaming\\", "\\appdata\\local\\", "\\appdata\\locallow\\"];

/// Files that mark a folder as a code project. Lowercase.
const PROJECT_MARKERS: &[&str] = &[
    "cargo.toml", "package.json", "pyproject.toml", "setup.py", "requirements.txt", "pipfile", "build.gradle",
    "build.gradle.kts", "settings.gradle", "settings.gradle.kts", "pom.xml", "pubspec.yaml", "go.mod", "podfile",
    "cmakelists.txt",
];
const PROJECT_EXTS: &[&str] = &[".sln", ".csproj", ".vbproj", ".fsproj"];

impl App {
    /// Starts the chosen heuristics on a background thread. Findings arrive in `heuristics://done`.
    pub fn run_heuristics(self: &Arc<Self>, scan_id: ScanId, kinds: Vec<HeuristicKind>) -> ApiResult<JobId> {
        let scan = self.scan(scan_id)?;
        let job_id = self.next_id();
        let app = self.clone();
        let settings = self.settings();
        std::thread::Builder::new()
            .name(format!("heuristics-{job_id}"))
            .spawn(move || {
                let cancel = AtomicBool::new(false);
                let mut findings = Vec::new();
                for kind in dedup(kinds) {
                    let progress = |done: u64, total: u64| {
                        app.emit(Event::HeuristicsProgress(HeuristicsProgress { job_id, kind, done, total }));
                    };
                    progress(0, 0);
                    let mut found = match kind {
                        HeuristicKind::Orphan => orphans(&scan.tree),
                        HeuristicKind::Stale => stale(&scan.tree, settings.stale_months),
                        HeuristicKind::Duplicates => duplicates(&scan.tree, &cancel, &progress),
                        HeuristicKind::OldProject => old_projects(&scan.tree, settings.old_project_months),
                    };
                    for f in &mut found {
                        if f.node_id.is_none() {
                            f.node_id = scan.tree.find_by_path(&f.path);
                        }
                    }
                    findings.append(&mut found);
                    progress(1, 1);
                }
                app.emit(Event::HeuristicsDone(HeuristicsDone { job_id, scan_id, findings }));
            })
            .map_err(|e| ApiError::with_detail("heuristics_failed", e.to_string()))?;
        Ok(job_id)
    }
}

fn dedup(kinds: Vec<HeuristicKind>) -> Vec<HeuristicKind> {
    let mut seen = HashSet::new();
    kinds.into_iter().filter(|k| seen.insert(*k)).collect()
}

fn dir_record(tree: &ScanTree, id: u32, path: &str) -> DirRecord {
    let node = tree.node(id);
    DirRecord {
        node_id: Some(id),
        path: path.to_string(),
        name: tree.name(id).to_string(),
        size: node.map_or(0, |n| n.total_size),
        modified: node.and_then(|n| n.modified()),
    }
}

/// Folders one or two levels below AppData\Roaming, Local, LocalLow and ProgramData.
pub(crate) fn appdata_dirs(tree: &ScanTree) -> Vec<DirRecord> {
    let mut out = Vec::new();
    tree.visit_dirs_and_files(&mut |e| {
        if !e.is_dir {
            return Visit::Continue;
        }
        let lower = e.path_lower;
        let rest = APPDATA_CONTAINERS
            .iter()
            .find_map(|c| lower.find(c).map(|i| &lower[i + c.len()..]))
            .or_else(|| lower.get(2..).and_then(|r| r.strip_prefix("\\programdata\\")));
        match rest {
            Some(rest) if !rest.is_empty() => {
                let depth = rest.matches('\\').count();
                if depth <= 1 {
                    out.push(dir_record(tree, e.id, e.path));
                }
                if depth >= 1 {
                    Visit::SkipChildren
                } else {
                    Visit::Continue
                }
            }
            _ => Visit::Continue,
        }
    });
    out
}

fn orphans(tree: &ScanTree) -> Vec<HeuristicFinding> {
    let dirs = appdata_dirs(tree);
    if dirs.is_empty() {
        return Vec::new();
    }
    let installed = fazasanj_heuristics::installed_programs();
    fazasanj_heuristics::find_orphans(&dirs, &installed)
}

fn stale(tree: &ScanTree, months: u32) -> Vec<HeuristicFinding> {
    let reliable = fazasanj_heuristics::access_time_reliable();
    let files: Vec<FileRecord> = tree
        .files()
        .filter(|f| f.size > 0 && !f.hardlink_dup)
        .map(|f| {
            // Only big files are worth a disk read for the access time.
            let accessed = if reliable && f.size >= STALE_MIN_BYTES {
                std::fs::metadata(&f.path).and_then(|m| m.accessed()).ok().and_then(system_time_ms)
            } else {
                None
            };
            FileRecord { node_id: Some(f.id), path: f.path, size: f.size, modified: f.modified, accessed }
        })
        .collect();
    fazasanj_heuristics::find_stale(&files, months, now_ms(), reliable, STALE_MIN_BYTES)
}

fn system_time_ms(t: std::time::SystemTime) -> Option<i64> {
    t.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_millis() as i64)
}

fn duplicates(tree: &ScanTree, cancel: &AtomicBool, progress: &(dyn Fn(u64, u64) + Sync)) -> Vec<HeuristicFinding> {
    let files: Vec<FileRecord> = tree
        .files()
        .filter(|f| f.size >= DUP_MIN_BYTES && !f.hardlink_dup)
        .map(|f| FileRecord { node_id: Some(f.id), path: f.path, size: f.size, modified: f.modified, accessed: None })
        .collect();
    fazasanj_heuristics::find_duplicates(files, DUP_MIN_BYTES, cancel, progress)
}

/// Folders that hold a project marker file (or a `.git` folder).
pub(crate) fn project_dirs(tree: &ScanTree) -> Vec<DirRecord> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    tree.visit_dirs_and_files(&mut |e| {
        if e.id == 0 {
            return Visit::Continue;
        }
        let name = e.path_lower.rsplit('\\').next().unwrap_or("");
        if e.is_dir && matches!(name, "node_modules" | "$recycle.bin" | "windows" | "appdata") {
            return Visit::SkipChildren;
        }
        let marker = if e.is_dir {
            name == ".git"
        } else {
            PROJECT_MARKERS.contains(&name) || PROJECT_EXTS.iter().any(|x| name.ends_with(x))
        };
        if marker {
            if let Some(parent) = tree.parent(e.id) {
                if seen.insert(parent) {
                    let cut = e.path.len().saturating_sub(e.name.len());
                    let path = e.path.get(..cut).unwrap_or("").trim_end_matches('\\');
                    out.push(dir_record(tree, parent, path));
                }
            }
            if e.is_dir {
                return Visit::SkipChildren;
            }
        }
        Visit::Continue
    });
    out
}

fn old_projects(tree: &ScanTree, months: u32) -> Vec<HeuristicFinding> {
    let dirs = project_dirs(tree);
    fazasanj_heuristics::find_old_projects(&dirs, months, now_ms())
}
