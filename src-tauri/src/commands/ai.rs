use fazasanj_ai::{keys, ApiKey, FolderMeta};
use fazasanj_model::{AiAnswer, AiProvider, AiProviderStatus, ApiError, Language, NodeId, ScanId};
use serde_json::Value;
use tauri::State;

use crate::state::AppState;

fn model_key(p: AiProvider) -> String {
    format!("ai.model.{}", fazasanj_ai::provider_id(p))
}

fn chosen_model(state: &AppState, p: AiProvider) -> String {
    state
        .store
        .get_value(&model_key(p))
        .ok()
        .flatten()
        .filter(|m| !m.trim().is_empty())
        .unwrap_or_else(|| fazasanj_ai::default_model(p).to_string())
}

fn key_for(p: AiProvider) -> Result<ApiKey, ApiError> {
    keys::get(p).map_err(|e| e.to_api_error())?.ok_or_else(|| ApiError::new("no_key"))
}

#[tauri::command]
pub fn ai_get_providers(state: State<'_, AppState>) -> Vec<AiProviderStatus> {
    fazasanj_ai::ALL_PROVIDERS
        .iter()
        .map(|&p| {
            let chosen = state.store.get_value(&model_key(p)).ok().flatten();
            fazasanj_ai::provider_status(p, chosen.as_deref())
        })
        .collect()
}

#[tauri::command]
pub fn ai_set_key(provider: AiProvider, key: String) -> Result<(), ApiError> {
    let key = key.trim();
    if key.is_empty() {
        return Err(ApiError::new("no_key"));
    }
    keys::set(provider, key).map_err(|e| e.to_api_error())
}

#[tauri::command]
pub fn ai_delete_key(provider: AiProvider) -> Result<(), ApiError> {
    keys::delete(provider).map_err(|e| e.to_api_error())
}

#[tauri::command]
pub fn ai_set_model(state: State<'_, AppState>, provider: AiProvider, model: String) -> Result<(), ApiError> {
    let model = model.trim();
    if model.is_empty() || model.len() > 100 {
        return Err(ApiError::with_detail("provider_error", "bad_model_name"));
    }
    state.store.set_value(&model_key(provider), model).map_err(|e| ApiError::with_detail(e.code(), e.to_string()))
}

#[tauri::command]
pub async fn ai_test_key(state: State<'_, AppState>, provider: AiProvider) -> Result<(), ApiError> {
    let key = key_for(provider)?;
    let model = chosen_model(&state, provider);
    fazasanj_ai::test_key(provider, &model, &key).await.map_err(|e| e.to_api_error())
}

fn username() -> String {
    std::env::var("USERNAME").unwrap_or_default()
}

fn payload_for(state: &AppState, scan_id: ScanId, node_id: NodeId) -> Result<(Value, String), ApiError> {
    let meta: FolderMeta = state.scans.folder_meta(scan_id, node_id)?;
    let path = meta.path.clone();
    let mask = state.settings().ai_mask_names;
    Ok((fazasanj_ai::build_payload(&meta, mask, &username()), path))
}

#[tauri::command]
pub fn ai_preview_payload(state: State<'_, AppState>, scan_id: ScanId, node_id: NodeId) -> Result<Value, ApiError> {
    payload_for(&state, scan_id, node_id).map(|(v, _)| v)
}

#[tauri::command]
pub async fn ai_explain(
    state: State<'_, AppState>,
    scan_id: ScanId,
    node_id: NodeId,
    language: Language,
) -> Result<AiAnswer, ApiError> {
    let settings = state.settings();
    if !settings.ai_enabled {
        return Err(ApiError::new("ai_disabled"));
    }
    let provider = settings.ai_default_provider.unwrap_or(AiProvider::OpenAi);
    let model = chosen_model(&state, provider);
    let (payload, real_path) = payload_for(&state, scan_id, node_id)?;
    let cache_key = fazasanj_ai::cache_key(provider, &model, &payload, language);
    if let Ok(Some(mut hit)) = state.store.ai_cache_get(&cache_key) {
        hit.path = real_path;
        hit.cached = true;
        return Ok(hit);
    }
    let key = key_for(provider)?;
    let mut answer = fazasanj_ai::explain(provider, &model, &key, &payload, language)
        .await
        .map_err(|e| e.to_api_error())?;
    answer.path = real_path;
    // Failing to cache is not worth failing the answer.
    let _ = state.store.ai_cache_put(&cache_key, &answer);
    Ok(answer)
}
