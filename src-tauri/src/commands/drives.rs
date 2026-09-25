use fazasanj_model::{ApiError, DriveInfo};

#[tauri::command]
pub fn list_drives() -> Result<Vec<DriveInfo>, ApiError> {
    fazasanj_platform::list_drives().map_err(|e| ApiError::with_detail("drives_failed", e.to_string()))
}
