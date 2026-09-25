use fazasanj_app::ApiResult;
use fazasanj_model::{
    AccessDeniedEntry, ChildSort, ChildrenPage, FileEntry, NodeId, NodeInfo, ScanId, ScanMode, ScanSummary,
    ScannerComparison, Story, TreemapNode, TypeGroup,
};

use super::{blocking, AppState};

#[tauri::command]
pub fn start_scan(state: AppState<'_>, path: String, mode: ScanMode) -> ApiResult<ScanId> {
    state.inner().start_scan(&path, mode)
}

#[tauri::command]
pub fn cancel_scan(state: AppState<'_>, scan_id: ScanId) {
    state.cancel_scan(scan_id);
}

#[tauri::command]
pub fn close_scan(state: AppState<'_>, scan_id: ScanId) {
    state.close_scan(scan_id);
}

#[tauri::command]
pub fn get_scan_summary(state: AppState<'_>, scan_id: ScanId) -> ApiResult<ScanSummary> {
    state.scan_summary(scan_id)
}

#[tauri::command]
pub async fn get_node(state: AppState<'_>, scan_id: ScanId, node_id: NodeId) -> ApiResult<NodeInfo> {
    let app = state.inner().clone();
    blocking(move || app.node(scan_id, node_id)).await
}

#[tauri::command]
pub async fn get_children(
    state: AppState<'_>,
    scan_id: ScanId,
    node_id: NodeId,
    sort: ChildSort,
    offset: u32,
    limit: u32,
) -> ApiResult<ChildrenPage> {
    let app = state.inner().clone();
    blocking(move || app.children(scan_id, node_id, sort, offset, limit)).await
}

#[tauri::command]
pub async fn get_treemap(
    state: AppState<'_>,
    scan_id: ScanId,
    node_id: NodeId,
    depth: u32,
    max_items: u32,
) -> ApiResult<TreemapNode> {
    let app = state.inner().clone();
    blocking(move || app.treemap(scan_id, node_id, depth, max_items)).await
}

#[tauri::command]
pub async fn get_largest_files(state: AppState<'_>, scan_id: ScanId, limit: u32) -> ApiResult<Vec<FileEntry>> {
    let app = state.inner().clone();
    blocking(move || app.largest_files(scan_id, limit)).await
}

#[tauri::command]
pub async fn get_by_type(state: AppState<'_>, scan_id: ScanId) -> ApiResult<Vec<TypeGroup>> {
    let app = state.inner().clone();
    blocking(move || app.by_type(scan_id)).await
}

#[tauri::command]
pub fn get_access_denied(state: AppState<'_>, scan_id: ScanId) -> ApiResult<Vec<AccessDeniedEntry>> {
    state.access_denied(scan_id)
}

#[tauri::command]
pub async fn get_story(state: AppState<'_>, scan_id: ScanId) -> ApiResult<Story> {
    let app = state.inner().clone();
    blocking(move || app.story(scan_id)).await
}

#[tauri::command]
pub async fn compare_scanners(state: AppState<'_>, path: String) -> ApiResult<ScannerComparison> {
    let app = state.inner().clone();
    blocking(move || app.compare_scanners(&path)).await
}

#[tauri::command]
pub fn run_heuristics(
    state: AppState<'_>,
    scan_id: ScanId,
    kinds: Vec<fazasanj_model::HeuristicKind>,
) -> ApiResult<fazasanj_model::JobId> {
    state.inner().run_heuristics(scan_id, kinds)
}
