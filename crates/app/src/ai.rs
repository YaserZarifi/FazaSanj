//! Optional AI explanations. Off unless the user turns it on, metadata only, cached.

use fazasanj_ai::{AiError, FolderMeta};
use fazasanj_model::{AiAnswer, AiProvider, AiProviderStatus, ApiError, Language, NodeId, ScanId};
use serde_json::Value;

use crate::{store_err, ApiResult, App};

const MAX_CHILD_NAMES: usize = 15;
const MAX_EXTENSIONS: usize = 10;

fn model_key(p: AiProvider) -> String {
    format!("ai_model.{}", fazasanj_ai::provider_id(p))
}

fn ai_err(e: AiError) -> ApiError {
    e.to_api_error()
}

impl App {
    pub fn ai_providers(&self) -> Vec<AiProviderStatus> {
        fazasanj_ai::ALL_PROVIDERS
            .iter()
            .map(|&p| {
                let chosen = self.store.get_value(&model_key(p)).ok().flatten();
                fazasanj_ai::provider_status(p, chosen.as_deref())
            })
            .collect()
    }

    fn ai_model(&self, p: AiProvider) -> String {
        self.store
            .get_value(&model_key(p))
            .ok()
            .flatten()
            .filter(|m| !m.trim().is_empty())
            .unwrap_or_else(|| fazasanj_ai::default_model(p).to_string())
    }

    pub fn ai_set_model(&self, p: AiProvider, model: &str) -> ApiResult<()> {
        let model = model.trim();
        if model.is_empty() || model.len() > 100 {
            return Err(ApiError::new("invalid_model"));
        }
        self.store.set_value(&model_key(p), model).map_err(store_err)
    }

    pub fn ai_set_key(&self, p: AiProvider, key: &str) -> ApiResult<()> {
        let key = key.trim();
        if key.is_empty() {
            return Err(ApiError::new("no_key"));
        }
        fazasanj_ai::keys::set(p, key).map_err(ai_err)
    }

    pub fn ai_delete_key(&self, p: AiProvider) -> ApiResult<()> {
        fazasanj_ai::keys::delete(p).map_err(ai_err)
    }

    pub async fn ai_test_key(&self, p: AiProvider) -> ApiResult<()> {
        let key = fazasanj_ai::keys::get(p).map_err(ai_err)?.ok_or_else(|| ai_err(AiError::NoKey))?;
        fazasanj_ai::test_key(p, &self.ai_model(p), &key).await.map_err(ai_err)
    }

    fn folder_meta(&self, scan_id: ScanId, node_id: NodeId) -> ApiResult<FolderMeta> {
        let scan = self.scan(scan_id)?;
        let tree = &scan.tree;
        let node = tree.node(node_id).ok_or_else(|| ApiError::new("node_not_found"))?;
        let (oldest, newest) = tree.subtree_date_range(node_id);
        Ok(FolderMeta {
            path: tree.path_of(node_id),
            total_bytes: node.total_size,
            file_count: u64::from(node.file_count),
            dir_count: u64::from(node.dir_count),
            top_extensions: tree.subtree_extension_stats(node_id, MAX_EXTENSIONS),
            oldest_modified: oldest,
            newest_modified: newest,
            child_names: tree.children(node_id).take(MAX_CHILD_NAMES).map(|c| tree.name(c).to_string()).collect(),
            parent_app_hint: None,
        })
    }

    /// The exact JSON that would be sent for this folder.
    pub fn ai_preview_payload(&self, scan_id: ScanId, node_id: NodeId) -> ApiResult<Value> {
        let meta = self.folder_meta(scan_id, node_id)?;
        Ok(fazasanj_ai::preview_payload(&meta, self.settings().ai_mask_names, &username()))
    }

    fn pick_provider(&self) -> ApiResult<AiProvider> {
        let settings = self.settings();
        if let Some(p) = settings.ai_default_provider {
            return Ok(p);
        }
        fazasanj_ai::ALL_PROVIDERS
            .iter()
            .copied()
            .find(|&p| fazasanj_ai::keys::has(p))
            .ok_or_else(|| ai_err(AiError::NoKey))
    }

    /// Asks the chosen provider about one folder, or returns the cached answer.
    pub async fn ai_explain(&self, scan_id: ScanId, node_id: NodeId, language: Language) -> ApiResult<AiAnswer> {
        let settings = self.settings();
        if !settings.ai_enabled {
            return Err(ApiError::new("ai_disabled"));
        }
        let meta = self.folder_meta(scan_id, node_id)?;
        let real_path = meta.path.clone();
        let payload = fazasanj_ai::build_payload(&meta, settings.ai_mask_names, &username());
        let provider = self.pick_provider()?;
        let model = self.ai_model(provider);
        let key = fazasanj_ai::cache_key(provider, &model, &payload, language);
        if let Ok(Some(mut cached)) = self.store.ai_cache_get(&key) {
            cached.cached = true;
            cached.path = real_path;
            return Ok(cached);
        }
        let api_key =
            fazasanj_ai::keys::get(provider).map_err(ai_err)?.ok_or_else(|| ai_err(AiError::NoKey))?;
        let mut answer = fazasanj_ai::explain(provider, &model, &api_key, &payload, language).await.map_err(ai_err)?;
        answer.path = real_path;
        if let Err(e) = self.store.ai_cache_put(&key, &answer) {
            log::warn!("could not cache AI answer: {e}");
        }
        Ok(answer)
    }
}

fn username() -> String {
    std::env::var("USERNAME").or_else(|_| std::env::var("USER")).unwrap_or_default()
}
