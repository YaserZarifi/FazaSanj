pub mod ai;
pub mod app;
pub mod cleanup;
pub mod drives;
pub mod scan;
pub mod snapshots;

use std::sync::Arc;

use fazasanj_app::{ApiResult, App};
use fazasanj_model::ApiError;

pub type AppState<'a> = tauri::State<'a, Arc<App>>;

/// Runs tree queries off the main thread so a big scan never freezes the window.
pub async fn blocking<T, F>(f: F) -> ApiResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> ApiResult<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| ApiError::with_detail("internal_error", e.to_string()))?
}
