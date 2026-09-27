use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;

use fazasanj_model::{
    AccessDeniedEntry, ApiError, ChildSort, ChildrenPage, FileEntry, NodeId, NodeInfo, ScanFailed, ScanId, ScanMode,
    ScanProgress, ScanSummary, ScannerComparison, Story, TreemapNode, TypeGroup,
};
use fazasanj_scan::{ScanError, ScanOptions};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::scans::{build_story, explain_all, explained, tag_tree, ScanSession};
use crate::state::{lock, now_ms, AppState};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Cancelled {
    scan_id: ScanId,
}

fn helper_path() -> Option<PathBuf> {
    fazasanj_platform::find_sidecar(fazasanj_platform::FAST_SCAN_HELPER)
}

fn scan_err(e: &ScanError) -> ApiError {
    ApiError::with_detail(e.code(), e.to_string())
}

#[tauri::command]
pub fn start_scan(app: AppHandle, state: State<'_, AppState>, path: String, mode: ScanMode) -> Result<ScanId, ApiError> {
    let root = PathBuf::from(path.trim());
    if !root.is_absolute() || !root.exists() {
        return Err(ApiError::with_detail("not_found", path));
    }
    let scan_id = state.next_id();
    let session = Arc::new(ScanSession::new(fazasanj_scan::display_path(&root)));
    state.scans.insert(scan_id, session.clone());

    let settings = state.settings();
    let progress_app = app.clone();
    let mut opts = ScanOptions::new(root.clone(), mode);
    opts.excluded = settings.excluded_paths.iter().map(PathBuf::from).collect();
    opts.helper_path = helper_path();
    opts.cancel = session.cancel.clone();
    opts.progress = Arc::new(move |p| {
        let _ = progress_app.emit(
            "scan://progress",
            ScanProgress {
                scan_id,
                files: p.files,
                dirs: p.dirs,
                bytes: p.bytes,
                current_path: p.current_path,
                elapsed_ms: p.elapsed_ms,
                scanner: p.scanner,
            },
        );
    });

    std::thread::Builder::new()
        .name(format!("scan-{scan_id}"))
        .spawn(move || run_scan_job(app, scan_id, session, root, opts))
        .map_err(|e| ApiError::with_detail("scan_failed", e.to_string()))?;
    Ok(scan_id)
}

fn run_scan_job(app: AppHandle, scan_id: ScanId, session: Arc<ScanSession>, root: PathBuf, opts: ScanOptions) {
    let state = app.state::<AppState>();
    let outcome = match fazasanj_scan::run_scan(opts) {
        Ok(o) => o,
        Err(ScanError::Cancelled) => {
            state.scans.remove(scan_id);
            let _ = app.emit("scan://cancelled", Cancelled { scan_id });
            return;
        }
        Err(e) => {
            state.scans.remove(scan_id);
            let _ = app.emit("scan://error", ScanFailed { scan_id, error: scan_err(&e) });
            return;
        }
    };

    let mut tree = outcome.tree;
    let matches = tag_tree(&mut tree, &state.rules, now_ms());
    let space = fazasanj_platform::disk_space(&root).ok();
    let ts = tree.summary();
    let summary = ScanSummary {
        scan_id,
        root_path: session.root.clone(),
        root_node: tree.root(),
        total_bytes: ts.total_bytes,
        files: ts.files,
        dirs: ts.dirs,
        access_denied: outcome.access_denied.len() as u32,
        cloud_only: ts.cloud_only,
        duration_ms: outcome.duration_ms,
        scanner: outcome.scanner_used,
        fallback_reason: outcome.fallback_reason,
        drive_total: space.as_ref().map(|s| s.total).unwrap_or(0),
        drive_free: space.as_ref().map(|s| s.free).unwrap_or(0),
        finished_at: now_ms(),
    };
    if let Some(d) = &outcome.fallback_detail {
        log::info!("fast scan fell back: {d}");
    }

    // Snapshot for growth tracking. Failing here must not fail the scan.
    let rows = tree.folder_sizes(4);
    match state.store.save_snapshot(
        &summary.root_path,
        summary.finished_at,
        summary.total_bytes,
        summary.files,
        summary.drive_total,
        summary.drive_free,
        &rows,
    ) {
        Ok(id) => *lock(&session.snapshot_id) = Some(id),
        Err(e) => log::warn!("snapshot not saved: {e}"),
    }
    let _ = state.store.prune_snapshots(30);
    let _ = state.store.set_last_scan(&summary);

    *lock(&session.access_denied) = outcome.access_denied;
    *lock(&session.matches) = matches;
    *lock(&session.summary) = Some(summary.clone());
    *session.tree.write().unwrap_or_else(|e| e.into_inner()) = Some(tree);
    state.scans.drop_older_than(scan_id, &session.root);
    let _ = app.emit("scan://done", summary);
}

#[tauri::command]
pub fn cancel_scan(state: State<'_, AppState>, scan_id: ScanId) -> Result<(), ApiError> {
    let s = state.scans.get(scan_id)?;
    s.cancel.store(true, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
pub fn get_scan_summary(state: State<'_, AppState>, scan_id: ScanId) -> Result<ScanSummary, ApiError> {
    let s = state.scans.get(scan_id)?;
    let summary = lock(&s.summary).clone();
    summary.ok_or_else(|| ApiError::new("scan_not_ready"))
}

#[tauri::command]
pub fn get_node(state: State<'_, AppState>, scan_id: ScanId, node_id: NodeId) -> Result<NodeInfo, ApiError> {
    let s = state.scans.get(scan_id)?;
    let tree = s.read()?;
    explained(&tree, &state.rules, node_id).ok_or_else(|| ApiError::new("not_found"))
}

#[tauri::command]
pub fn get_children(
    state: State<'_, AppState>,
    scan_id: ScanId,
    node_id: NodeId,
    sort: ChildSort,
    offset: u32,
    limit: u32,
) -> Result<ChildrenPage, ApiError> {
    let s = state.scans.get(scan_id)?;
    let tree = s.read()?;
    let mut page = tree
        .children_page(node_id, sort, offset, limit.clamp(1, 1000))
        .ok_or_else(|| ApiError::new("not_found"))?;
    explain_all(&tree, &state.rules, &mut page.items);
    Ok(page)
}

#[tauri::command]
pub fn get_treemap(
    state: State<'_, AppState>,
    scan_id: ScanId,
    node_id: NodeId,
    depth: u32,
    max_items: u32,
) -> Result<TreemapNode, ApiError> {
    let s = state.scans.get(scan_id)?;
    let tree = s.read()?;
    tree.treemap(node_id, depth.clamp(1, 3), max_items.clamp(10, 400))
        .ok_or_else(|| ApiError::new("not_found"))
}

#[tauri::command]
pub fn get_largest_files(state: State<'_, AppState>, scan_id: ScanId, limit: usize) -> Result<Vec<FileEntry>, ApiError> {
    let s = state.scans.get(scan_id)?;
    let tree = s.read()?;
    Ok(tree.largest_files(limit.clamp(1, 1000)))
}

#[tauri::command]
pub fn get_by_type(state: State<'_, AppState>, scan_id: ScanId) -> Result<Vec<TypeGroup>, ApiError> {
    let s = state.scans.get(scan_id)?;
    let tree = s.read()?;
    Ok(tree.by_type())
}

#[tauri::command]
pub fn get_access_denied(state: State<'_, AppState>, scan_id: ScanId) -> Result<Vec<AccessDeniedEntry>, ApiError> {
    let s = state.scans.get(scan_id)?;
    let list = lock(&s.access_denied).iter().map(|p| AccessDeniedEntry { path: p.clone() }).collect();
    Ok(list)
}

#[tauri::command]
pub fn get_story(state: State<'_, AppState>, scan_id: ScanId) -> Result<Story, ApiError> {
    let s = state.scans.get(scan_id)?;
    let summary = lock(&s.summary).clone().ok_or_else(|| ApiError::new("scan_not_ready"))?;
    let tree = s.read()?;
    let matches = lock(&s.matches).clone();
    Ok(build_story(&tree, &state.rules, &summary, &matches))
}

#[tauri::command]
pub async fn compare_scanners(state: State<'_, AppState>, path: String) -> Result<ScannerComparison, ApiError> {
    let root = PathBuf::from(path.trim());
    if !Path::new(&root).exists() {
        return Err(ApiError::new("not_found"));
    }
    let (job, cancel) = state.start_job();
    let result =
        tauri::async_runtime::spawn_blocking(move || fazasanj_scan::compare_scanners(&root, helper_path(), cancel)).await;
    state.end_job(job);
    result
        .map_err(|e| ApiError::with_detail("scan_failed", e.to_string()))?
        .map_err(|e| scan_err(&e))
}
