use std::collections::HashSet;

use fazasanj_heuristics::{DirRecord, FileRecord};
use fazasanj_model::{
    ApiError, HeuristicFinding, HeuristicKind, HeuristicsDone, HeuristicsProgress, JobId, ScanId,
};
use fazasanj_scan::{ScanTree, Visit};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::state::{now_ms, AppState};

const PROJECT_MARKERS: [&str; 12] = [
    "package.json",
    "cargo.toml",
    ".git",
    "pyproject.toml",
    "setup.py",
    "requirements.txt",
    "build.gradle",
    "build.gradle.kts",
    "pom.xml",
    "go.mod",
    "composer.json",
    "pubspec.yaml",
];

const APPDATA_PARENTS: [&str; 3] = ["\\appdata\\roaming", "\\appdata\\local", "\\appdata\\locallow"];

fn parent_of(path: &str) -> &str {
    path.rfind('\\').map(|i| &path[..i]).unwrap_or(path)
}

/// Folders directly under AppData (and ProgramData), plus one level below for vendor folders.
fn appdata_dirs(tree: &ScanTree) -> Vec<DirRecord> {
    let mut out = Vec::new();
    tree.visit_dirs_and_files(&mut |e| {
        if !e.is_dir {
            return Visit::SkipChildren;
        }
        let parent = parent_of(e.path_lower);
        let grand = parent_of(parent);
        let is_programdata = |p: &str| p.len() == 14 && p.ends_with(":\\programdata");
        let direct = APPDATA_PARENTS.iter().any(|a| parent.ends_with(a)) || is_programdata(parent);
        let nested = APPDATA_PARENTS.iter().any(|a| grand.ends_with(a)) || is_programdata(grand);
        if direct || nested {
            out.push(DirRecord {
                node_id: Some(e.id),
                path: e.path.to_string(),
                name: e.name.to_string(),
                size: e.size,
                modified: e.modified,
            });
        }
        if nested {
            Visit::SkipChildren
        } else {
            Visit::Continue
        }
    });
    out
}

fn project_dirs(tree: &ScanTree) -> Vec<DirRecord> {
    let mut parents: HashSet<String> = HashSet::new();
    tree.visit_dirs_and_files(&mut |e| {
        let lower_name = e.name.to_lowercase();
        if matches!(lower_name.as_str(), "node_modules" | "$recycle.bin" | "windows" | "program files")
            && e.is_dir
        {
            return Visit::SkipChildren;
        }
        if PROJECT_MARKERS.contains(&lower_name.as_str()) || lower_name.ends_with(".sln") {
            parents.insert(parent_of(e.path).to_string());
        }
        if e.is_dir && lower_name == ".git" {
            return Visit::SkipChildren;
        }
        Visit::Continue
    });
    parents
        .into_iter()
        .filter_map(|p| {
            let id = tree.find_by_path(&p)?;
            let n = tree.node(id)?;
            Some(DirRecord {
                node_id: Some(id),
                name: tree.name(id).to_string(),
                path: p,
                size: n.total_size,
                modified: n.modified(),
            })
        })
        .collect()
}

/// The tree stores allocated size, the duplicate finder compares real lengths. Only files that
/// share an allocated size can be copies, so only those get a (cheap) metadata call.
fn duplicate_candidates(tree: &ScanTree, min_size: u64) -> Vec<FileRecord> {
    let mut by_alloc: std::collections::HashMap<u64, Vec<FileRecord>> = std::collections::HashMap::new();
    for f in files(tree, min_size) {
        by_alloc.entry(f.size).or_default().push(f);
    }
    by_alloc
        .into_values()
        .filter(|g| g.len() >= 2)
        .flatten()
        .filter_map(|mut f| {
            f.size = std::fs::metadata(&f.path).ok()?.len();
            Some(f)
        })
        .collect()
}

fn files(tree: &ScanTree, min_size: u64) -> Vec<FileRecord> {
    tree.files()
        .filter(|f| !f.hardlink_dup && f.size >= min_size)
        .map(|f| FileRecord { node_id: Some(f.id), path: f.path, size: f.size, modified: f.modified, accessed: None })
        .collect()
}

#[tauri::command]
pub fn run_heuristics(
    app: AppHandle,
    state: State<'_, AppState>,
    scan_id: ScanId,
    kinds: Vec<HeuristicKind>,
) -> Result<JobId, ApiError> {
    let session = state.scans.get(scan_id)?;
    session.read()?;
    let (job_id, cancel) = state.start_job();
    std::thread::spawn(move || {
        let state = app.state::<AppState>();
        let settings = state.settings();
        let now = now_ms();
        let mut findings: Vec<HeuristicFinding> = Vec::new();
        let emit = |kind: HeuristicKind, done: u64, total: u64| {
            let _ = app.emit("heuristics://progress", HeuristicsProgress { job_id, kind, done, total });
        };
        if let Ok(tree) = session.read() {
            for kind in kinds {
                emit(kind, 0, 1);
                let mut found = match kind {
                    HeuristicKind::Orphan => {
                        let installed = fazasanj_heuristics::installed_programs();
                        fazasanj_heuristics::find_orphans(&appdata_dirs(&tree), &installed)
                    }
                    HeuristicKind::Stale => fazasanj_heuristics::find_stale(
                        &files(&tree, 1024 * 1024),
                        settings.stale_months,
                        now,
                        fazasanj_heuristics::access_time_reliable(),
                        1024 * 1024,
                    ),
                    HeuristicKind::Duplicates => {
                        let progress = |done: u64, total: u64| emit(HeuristicKind::Duplicates, done, total);
                        fazasanj_heuristics::find_duplicates(duplicate_candidates(&tree, 1024 * 1024), 1024 * 1024, &cancel, &progress)
                    }
                    HeuristicKind::OldProject => {
                        fazasanj_heuristics::find_old_projects(&project_dirs(&tree), settings.old_project_months, now)
                    }
                };
                for f in &mut found {
                    if f.node_id.is_none() {
                        f.node_id = tree.find_by_path(&f.path);
                    }
                }
                findings.append(&mut found);
                emit(kind, 1, 1);
                if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }
            }
        }
        state.end_job(job_id);
        let _ = app.emit("heuristics://done", HeuristicsDone { job_id, scan_id, findings });
    });
    Ok(job_id)
}
