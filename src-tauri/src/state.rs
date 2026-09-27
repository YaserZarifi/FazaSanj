use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use fazasanj_cleanup::CleanupEngine;
use fazasanj_model::{AppSettings, CleanupPlan, JobId};
use fazasanj_rules::RuleSet;
use fazasanj_safety::SafetyContext;
use fazasanj_store::Store;

use crate::scans::ScanRegistry;

/// Everything the commands share. Lives in Tauri's managed state.
pub struct AppState {
    pub store: Store,
    pub rules: Arc<RuleSet>,
    pub safety: SafetyContext,
    pub data_dir: PathBuf,
    pub scans: ScanRegistry,
    ids: AtomicU32,
    plans: Mutex<HashMap<u32, CleanupPlan>>,
    jobs: Mutex<HashMap<JobId, Arc<AtomicBool>>>,
}

impl AppState {
    pub fn new(store: Store, rules: RuleSet, safety: SafetyContext, data_dir: PathBuf) -> Self {
        Self {
            store,
            rules: Arc::new(rules),
            safety,
            data_dir,
            scans: ScanRegistry::default(),
            ids: AtomicU32::new(1),
            plans: Mutex::new(HashMap::new()),
            jobs: Mutex::new(HashMap::new()),
        }
    }

    pub fn next_id(&self) -> u32 {
        self.ids.fetch_add(1, Ordering::Relaxed)
    }

    pub fn settings(&self) -> AppSettings {
        self.store.get_settings().unwrap_or_default()
    }

    pub fn cleanup_engine(&self) -> CleanupEngine {
        let sandbox = self.settings().dev_sandbox.filter(|s| !s.trim().is_empty()).map(PathBuf::from);
        CleanupEngine::new(self.safety.clone(), sandbox, cfg!(debug_assertions))
    }

    pub fn put_plan(&self, plan: CleanupPlan) {
        lock(&self.plans).insert(plan.plan_id, plan);
    }

    pub fn plan(&self, id: u32) -> Option<CleanupPlan> {
        lock(&self.plans).get(&id).cloned()
    }

    pub fn start_job(&self) -> (JobId, Arc<AtomicBool>) {
        let id = self.next_id();
        let flag = Arc::new(AtomicBool::new(false));
        lock(&self.jobs).insert(id, flag.clone());
        (id, flag)
    }

    pub fn end_job(&self, id: JobId) {
        lock(&self.jobs).remove(&id);
    }
}

/// Locks a mutex, recovering the data if another thread panicked while holding it.
pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
