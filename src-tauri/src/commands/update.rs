use fazasanj_model::ApiError;
use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub notes: Option<String>,
}

#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<Option<UpdateInfo>, ApiError> {
    let updater = app.updater().map_err(|e| ApiError::with_detail("update_failed", e.to_string()))?;
    match updater.check().await {
        Ok(Some(u)) => Ok(Some(UpdateInfo { version: u.version.clone(), notes: u.body.clone() })),
        Ok(None) => Ok(None),
        // Offline or no release yet, nothing to tell the user.
        Err(e) => {
            log::info!("update check failed: {e}");
            Ok(None)
        }
    }
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), ApiError> {
    let updater = app.updater().map_err(|e| ApiError::with_detail("update_failed", e.to_string()))?;
    let update = updater
        .check()
        .await
        .map_err(|e| ApiError::with_detail("update_failed", e.to_string()))?
        .ok_or_else(|| ApiError::new("no_update"))?;
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| ApiError::with_detail("update_failed", e.to_string()))?;
    app.restart();
}
