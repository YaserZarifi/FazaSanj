use std::path::{Path, PathBuf};

use fazasanj_app::{checked_open_target, ApiResult};
use fazasanj_model::{ApiError, CleanupOptions, CleanupPlan, CleanupTarget, HistoryEntry, JobId};
use tauri_plugin_opener::OpenerExt;

use super::AppState;

#[tauri::command]
pub fn build_cleanup_plan(state: AppState<'_>, targets: Vec<CleanupTarget>) -> ApiResult<CleanupPlan> {
    state.build_cleanup_plan(targets)
}

#[tauri::command]
pub fn run_cleanup(state: AppState<'_>, plan_id: u32, options: CleanupOptions) -> ApiResult<JobId> {
    state.inner().run_cleanup(plan_id, options)
}

#[tauri::command]
pub fn get_cleanup_history(state: AppState<'_>, limit: u32, offset: u32) -> ApiResult<Vec<HistoryEntry>> {
    state.cleanup_history(limit, offset)
}

fn open_err(e: impl std::fmt::Display) -> ApiError {
    ApiError::with_detail("open_failed", e.to_string())
}

#[tauri::command]
pub fn reveal_in_explorer(app: tauri::AppHandle, path: String) -> ApiResult<()> {
    let p = Path::new(&path);
    if !p.is_absolute() {
        return Err(ApiError::new("not_found"));
    }
    app.opener().reveal_item_in_dir(p).map_err(open_err)
}

#[tauri::command]
pub fn open_recycle_bin() -> ApiResult<()> {
    std::process::Command::new(windows_dir().join("explorer.exe"))
        .arg("shell:RecycleBinFolder")
        .spawn()
        .map(|_| ())
        .map_err(open_err)
}

/// Opens a Windows settings page or one of the allowed system tools.
#[tauri::command]
pub fn open_app_setting(app: tauri::AppHandle, target: String) -> ApiResult<()> {
    let target = checked_open_target(&target)?;
    if target.starts_with("ms-settings:") {
        app.opener().open_url(target, None::<&str>).map_err(open_err)
    } else {
        let full = system32().join(&target);
        app.opener().open_path(full.to_string_lossy(), None::<&str>).map_err(open_err)
    }
}

fn windows_dir() -> PathBuf {
    PathBuf::from(std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string()))
}

fn system32() -> PathBuf {
    windows_dir().join("System32")
}
