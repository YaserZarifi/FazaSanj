//! Everything the app does between the UI and the core crates: open scans, rule tags, the Simple
//! mode story, heuristics jobs, cleanup plans, AI answers and snapshots.
//!
//! `src-tauri` only turns commands into calls on [`App`] and [`Event`]s into Tauri events, so
//! this crate can be tested without a window.

mod ai;
mod cleanup;
mod events;
mod heuristics;
pub mod notices;
mod scans;
mod settings;
mod snapshots;
mod story;
mod tagging;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use fazasanj_model::{ApiError, CleanupPlan, ScanId};
use fazasanj_rules::RuleSet;
use fazasanj_store::Store;

pub use cleanup::checked_open_target;
pub use events::{Event, EventSink};
pub use scans::ScanEntry;
pub use story::build_story;
pub use tagging::{explanation_for, tag_tree};

pub type ApiResult<T> = Result<T, ApiError>;

/// Snapshots kept per scanned root.
const SNAPSHOTS_PER_ROOT: usize = 30;
/// Folder depth stored in a snapshot (root is 0).
const SNAPSHOT_DEPTH: u32 = 4;

pub struct App {
    rules: RuleSet,
    store: Store,
    sink: EventSink,
    helper_path: Option<PathBuf>,
    install_dir: Option<PathBuf>,
    scans: Mutex<HashMap<ScanId, Arc<ScanEntry>>>,
    running: Mutex<HashMap<ScanId, Arc<AtomicBool>>>,
    plans: Mutex<HashMap<u32, CleanupPlan>>,
    next_id: AtomicU32,
}

impl std::fmt::Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App").field("rules", &self.rules.len()).finish_non_exhaustive()
    }
}

impl App {
    /// `helper_path` is the fast scan sidecar (None to look next to the exe), `install_dir`
    /// is added to the block list so the app can never delete itself.
    pub fn new(
        store: Store,
        sink: EventSink,
        helper_path: Option<PathBuf>,
        install_dir: Option<PathBuf>,
    ) -> ApiResult<Arc<App>> {
        let rules = RuleSet::load_embedded().map_err(|e| ApiError::with_detail("rules_invalid", e.to_string()))?;
        Ok(Arc::new(App {
            rules,
            store,
            sink,
            helper_path,
            install_dir,
            scans: Mutex::new(HashMap::new()),
            running: Mutex::new(HashMap::new()),
            plans: Mutex::new(HashMap::new()),
            next_id: AtomicU32::new(1),
        }))
    }

    pub fn rules(&self) -> &RuleSet {
        &self.rules
    }

    pub fn store(&self) -> &Store {
        &self.store
    }

    fn next_id(&self) -> u32 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    fn emit(&self, event: Event) {
        (self.sink)(event);
    }
}

pub(crate) fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

pub(crate) fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

pub(crate) fn store_err(e: fazasanj_store::StoreError) -> ApiError {
    ApiError::with_detail(e.code(), e.to_string())
}
