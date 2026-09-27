use fazasanj_model::{ApiError, AppSettings};
use tauri::State;

use crate::state::AppState;

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, ApiError> {
    state.store.get_settings().map_err(|e| ApiError::with_detail(e.code(), e.to_string()))
}

#[tauri::command]
pub fn set_settings(app: tauri::AppHandle, state: State<'_, AppState>, settings: AppSettings) -> Result<AppSettings, ApiError> {
    let mut s = settings;
    s.stale_months = s.stale_months.clamp(1, 120);
    s.old_project_months = s.old_project_months.clamp(1, 120);
    s.low_space_threshold_gb = s.low_space_threshold_gb.clamp(1, 10_000);
    s.excluded_paths.retain(|p| !p.trim().is_empty());
    s.excluded_paths.dedup();
    if !cfg!(debug_assertions) {
        s.dev_sandbox = None;
    }
    state.store.set_settings(&s).map_err(|e| ApiError::with_detail(e.code(), e.to_string()))?;
    crate::tray::apply(&app, &s);
    Ok(s)
}

/// Deletes stored data, AI keys and cached answers. Files on disk are never touched.
#[tauri::command]
pub fn reset_everything(state: State<'_, AppState>) -> Result<(), ApiError> {
    for p in fazasanj_ai::ALL_PROVIDERS {
        // A missing key is fine here.
        let _ = fazasanj_ai::keys::delete(p);
    }
    state.store.reset_everything().map_err(|e| ApiError::with_detail(e.code(), e.to_string()))?;
    state.scans.clear();
    Ok(())
}
