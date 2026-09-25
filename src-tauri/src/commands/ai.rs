use fazasanj_app::ApiResult;
use fazasanj_model::{AiAnswer, AiProvider, AiProviderStatus, Language, NodeId, ScanId};

use super::{blocking, AppState};

#[tauri::command]
pub fn ai_get_providers(state: AppState<'_>) -> Vec<AiProviderStatus> {
    state.ai_providers()
}

#[tauri::command]
pub fn ai_set_key(state: AppState<'_>, provider: AiProvider, key: String) -> ApiResult<()> {
    state.ai_set_key(provider, &key)
}

#[tauri::command]
pub fn ai_delete_key(state: AppState<'_>, provider: AiProvider) -> ApiResult<()> {
    state.ai_delete_key(provider)
}

#[tauri::command]
pub fn ai_set_model(state: AppState<'_>, provider: AiProvider, model: String) -> ApiResult<()> {
    state.ai_set_model(provider, &model)
}

#[tauri::command]
pub async fn ai_test_key(state: AppState<'_>, provider: AiProvider) -> ApiResult<()> {
    let app = state.inner().clone();
    app.ai_test_key(provider).await
}

#[tauri::command]
pub async fn ai_preview_payload(state: AppState<'_>, scan_id: ScanId, node_id: NodeId) -> ApiResult<serde_json::Value> {
    let app = state.inner().clone();
    blocking(move || app.ai_preview_payload(scan_id, node_id)).await
}

#[tauri::command]
pub async fn ai_explain(state: AppState<'_>, scan_id: ScanId, node_id: NodeId, language: Language) -> ApiResult<AiAnswer> {
    let app = state.inner().clone();
    app.ai_explain(scan_id, node_id, language).await
}
