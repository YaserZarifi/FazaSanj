use fazasanj_app::ApiResult;
use fazasanj_model::{GrowthPoint, ScanId, SnapshotComparison, SnapshotInfo};

use super::{blocking, AppState};

#[tauri::command]
pub fn list_snapshots(state: AppState<'_>, root_path: Option<String>) -> ApiResult<Vec<SnapshotInfo>> {
    state.list_snapshots(root_path.as_deref())
}

#[tauri::command]
pub async fn compare_snapshots(state: AppState<'_>, from_id: i64, to_id: i64) -> ApiResult<SnapshotComparison> {
    let app = state.inner().clone();
    blocking(move || app.compare_snapshots(from_id, to_id)).await
}

#[tauri::command]
pub async fn compare_with_last(state: AppState<'_>, scan_id: ScanId) -> ApiResult<Option<SnapshotComparison>> {
    let app = state.inner().clone();
    blocking(move || app.compare_with_last(scan_id)).await
}

#[tauri::command]
pub fn get_growth(state: AppState<'_>, path: String) -> ApiResult<Vec<GrowthPoint>> {
    state.growth(&path)
}
