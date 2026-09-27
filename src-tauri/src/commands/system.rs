use std::path::Path;
use std::process::Command;

use fazasanj_model::{ApiError, AppInfo};
use tauri::{AppHandle, State};

use crate::state::AppState;

#[tauri::command]
pub fn get_app_info(app: AppHandle, state: State<'_, AppState>) -> Result<AppInfo, ApiError> {
    let data_dir = &state.data_dir;
    Ok(AppInfo {
        version: app.package_info().version.to_string(),
        data_dir: data_dir.to_string_lossy().into_owned(),
        is_elevated: fazasanj_platform::is_elevated(),
        debug_build: cfg!(debug_assertions),
    })
}

/// Opens Explorer with the item selected (or the folder itself when it is a drive root).
#[tauri::command]
pub fn reveal_in_explorer(path: String) -> Result<(), ApiError> {
    let p = Path::new(&path);
    if !p.is_absolute() {
        return Err(ApiError::new("not_found"));
    }
    let mut cmd = Command::new("explorer.exe");
    if p.exists() && p.parent().is_some() {
        cmd.arg(format!("/select,{}", p.display()));
    } else if p.exists() {
        cmd.arg(p);
    } else {
        return Err(ApiError::new("not_found"));
    }
    cmd.spawn().map(|_| ()).map_err(|e| ApiError::with_detail("open_failed", e.to_string()))
}

#[tauri::command]
pub fn open_recycle_bin() -> Result<(), ApiError> {
    Command::new("explorer.exe")
        .arg("shell:RecycleBinFolder")
        .spawn()
        .map(|_| ())
        .map_err(|e| ApiError::with_detail("open_failed", e.to_string()))
}

/// Exes that rules may open to let the user handle something themselves.
const ALLOWED_EXES: [&str; 5] = [
    "SystemPropertiesProtection.exe",
    "SystemPropertiesPerformance.exe",
    "cleanmgr.exe",
    "control.exe",
    "explorer.exe",
];

/// Opens a Windows settings page (`ms-settings:...`) or one of a few known system tools.
#[tauri::command]
pub fn open_app_setting(target: String) -> Result<(), ApiError> {
    let t = target.trim();
    let is_settings_uri = t == "tg://settings"
        || t.starts_with("ms-settings:")
        && t.len() < 80
            && t["ms-settings:".len()..].chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    if is_settings_uri {
        return Command::new("explorer.exe")
            .arg(t)
            .spawn()
            .map(|_| ())
            .map_err(|e| ApiError::with_detail("open_failed", e.to_string()));
    }
    if let Some(exe) = ALLOWED_EXES.iter().find(|e| e.eq_ignore_ascii_case(t)) {
        return Command::new(exe)
            .spawn()
            .map(|_| ())
            .map_err(|e| ApiError::with_detail("open_failed", e.to_string()));
    }
    Err(ApiError::with_detail("not_allowed", t.to_string()))
}
