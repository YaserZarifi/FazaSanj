use fazasanj_app::ApiResult;
use fazasanj_model::{AppInfo, AppSettings};
use tauri::Manager;

use super::AppState;

#[tauri::command]
pub fn get_settings(state: AppState<'_>) -> AppSettings {
    state.settings()
}

#[tauri::command]
pub fn set_settings(app: tauri::AppHandle, state: AppState<'_>, settings: AppSettings) -> ApiResult<AppSettings> {
    let saved = state.set_settings(settings)?;
    crate::tray::sync(&app, saved.tray_enabled);
    Ok(saved)
}

#[tauri::command]
pub fn reset_everything(state: AppState<'_>) -> ApiResult<()> {
    state.reset_everything()
}

#[tauri::command]
pub fn get_app_info(app: tauri::AppHandle) -> AppInfo {
    AppInfo {
        version: app.package_info().version.to_string(),
        data_dir: app.path().app_data_dir().map(|p| p.display().to_string()).unwrap_or_default(),
        is_elevated: is_elevated(),
        debug_build: cfg!(debug_assertions),
    }
}

#[cfg(windows)]
fn is_elevated() -> bool {
    fazasanj_platform::is_elevated()
}

#[cfg(not(windows))]
fn is_elevated() -> bool {
    false
}
