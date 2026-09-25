//! Running scans and the finished trees kept in memory.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use fazasanj_model::{
    AccessDeniedEntry, ApiError, ChildSort, ChildrenPage, FileEntry, NodeId, NodeInfo, ScanFailed, ScanId, ScanMode,
    ScanProgress, ScanSummary, ScannerComparison, TreemapNode, TypeGroup,
};
use fazasanj_scan::{run_scan, ScanError, ScanOptions, ScanOutcome, ScanTree};

use crate::tagging::{explanation_for, tag_tree};
use crate::{lock, now_ms, ApiResult, App, Event, SNAPSHOTS_PER_ROOT, SNAPSHOT_DEPTH};

/// Most children the UI may ask for in one page.
const MAX_PAGE: u32 = 500;

/// One finished scan. Read only once it is published.
#[derive(Debug)]
pub struct ScanEntry {
    pub tree: ScanTree,
    pub summary: ScanSummary,
    pub access_denied: Vec<String>,
    pub snapshot_id: Option<i64>,
}

impl App {
    /// Starts a scan on a background thread. Progress and the result arrive as events.
    pub fn start_scan(self: &Arc<Self>, path: &str, mode: ScanMode) -> ApiResult<ScanId> {
        let path = path.trim();
        if path.is_empty() {
            return Err(ApiError::new("scan_root_not_found"));
        }
        let scan_id = self.next_id();
        let cancel = Arc::new(AtomicBool::new(false));
        lock(&self.running).insert(scan_id, cancel.clone());

        let settings = self.settings();
        let mut opts = ScanOptions::new(PathBuf::from(path), mode);
        opts.excluded = settings.excluded_paths.iter().map(PathBuf::from).collect();
        opts.helper_path = self.helper_path.clone();
        opts.cancel = cancel;
        let sink = self.sink.clone();
        opts.progress = Arc::new(move |p| {
            sink(Event::ScanProgress(ScanProgress {
                scan_id,
                files: p.files,
                dirs: p.dirs,
                bytes: p.bytes,
                current_path: p.current_path,
                elapsed_ms: p.elapsed_ms,
                scanner: p.scanner,
            }))
        });

        let app = self.clone();
        let spawned = std::thread::Builder::new().name(format!("scan-{scan_id}")).spawn(move || {
            let result = run_scan(opts);
            lock(&app.running).remove(&scan_id);
            match result {
                Ok(outcome) => {
                    let summary = app.publish(scan_id, outcome);
                    app.emit(Event::ScanDone(summary));
                }
                Err(ScanError::Cancelled) => app.emit(Event::ScanCancelled(scan_id)),
                Err(e) => {
                    log::warn!("scan {scan_id} failed: {e}");
                    app.emit(Event::ScanError(ScanFailed {
                        scan_id,
                        error: ApiError::with_detail(e.code(), e.to_string()),
                    }));
                }
            }
        });
        if let Err(e) = spawned {
            lock(&self.running).remove(&scan_id);
            return Err(ApiError::with_detail("scan_failed", e.to_string()));
        }
        Ok(scan_id)
    }

    pub fn cancel_scan(&self, scan_id: ScanId) {
        if let Some(flag) = lock(&self.running).get(&scan_id) {
            flag.store(true, Ordering::Relaxed);
        }
    }

    /// Tags the tree, saves a snapshot and keeps it as the only open scan of its root.
    fn publish(&self, scan_id: ScanId, outcome: ScanOutcome) -> ScanSummary {
        let ScanOutcome { mut tree, scanner_used, fallback_reason, duration_ms, access_denied, .. } = outcome;
        let now = now_ms();
        let tagged = tag_tree(&mut tree, &self.rules, now);
        log::info!("scan {scan_id}: {} nodes, {tagged} tagged", tree.len());

        let root_path = tree.root_path().to_string();
        let space = drive_space(Path::new(&root_path));
        let s = tree.summary();
        let summary = ScanSummary {
            scan_id,
            root_path: root_path.clone(),
            root_node: tree.root(),
            total_bytes: s.total_bytes,
            files: s.files,
            dirs: s.dirs,
            access_denied: access_denied.len() as u32,
            cloud_only: s.cloud_only,
            duration_ms,
            scanner: scanner_used,
            fallback_reason,
            drive_total: space.0,
            drive_free: space.1,
            finished_at: now,
        };
        if let Err(e) = self.store.set_last_scan(&summary) {
            log::warn!("could not save last scan: {e}");
        }
        let rows = tree.folder_sizes(SNAPSHOT_DEPTH);
        let snapshot_id =
            match self.store.save_snapshot(&root_path, now, s.total_bytes, s.files, space.0, space.1, &rows) {
                Ok(id) => Some(id),
                Err(e) => {
                    log::warn!("could not save snapshot: {e}");
                    None
                }
            };
        if let Err(e) = self.store.prune_snapshots(SNAPSHOTS_PER_ROOT) {
            log::warn!("could not prune snapshots: {e}");
        }

        let entry = Arc::new(ScanEntry { tree, summary: summary.clone(), access_denied, snapshot_id });
        let mut scans = lock(&self.scans);
        scans.retain(|_, e| !e.summary.root_path.eq_ignore_ascii_case(&root_path));
        scans.insert(scan_id, entry);
        summary
    }

    pub fn scan(&self, scan_id: ScanId) -> ApiResult<Arc<ScanEntry>> {
        lock(&self.scans).get(&scan_id).cloned().ok_or_else(|| ApiError::new("scan_not_found"))
    }

    /// Closes a scan and frees its memory.
    pub fn close_scan(&self, scan_id: ScanId) {
        lock(&self.scans).remove(&scan_id);
    }

    pub fn scan_summary(&self, scan_id: ScanId) -> ApiResult<ScanSummary> {
        Ok(self.scan(scan_id)?.summary.clone())
    }

    pub fn node(&self, scan_id: ScanId, node_id: NodeId) -> ApiResult<NodeInfo> {
        let scan = self.scan(scan_id)?;
        let mut info = scan.tree.node_info(node_id).ok_or_else(|| ApiError::new("node_not_found"))?;
        info.explanation = explanation_for(&scan.tree, &self.rules, node_id);
        Ok(info)
    }

    pub fn children(
        &self,
        scan_id: ScanId,
        node_id: NodeId,
        sort: ChildSort,
        offset: u32,
        limit: u32,
    ) -> ApiResult<ChildrenPage> {
        let scan = self.scan(scan_id)?;
        let mut page = scan
            .tree
            .children_page(node_id, sort, offset, limit.clamp(1, MAX_PAGE))
            .ok_or_else(|| ApiError::new("node_not_found"))?;
        for item in &mut page.items {
            item.explanation = explanation_for(&scan.tree, &self.rules, item.id);
        }
        Ok(page)
    }

    pub fn treemap(&self, scan_id: ScanId, node_id: NodeId, depth: u32, max_items: u32) -> ApiResult<TreemapNode> {
        let scan = self.scan(scan_id)?;
        scan.tree.treemap(node_id, depth.clamp(1, 3), max_items.clamp(5, 400)).ok_or_else(|| ApiError::new("node_not_found"))
    }

    pub fn largest_files(&self, scan_id: ScanId, limit: u32) -> ApiResult<Vec<FileEntry>> {
        Ok(self.scan(scan_id)?.tree.largest_files(limit.clamp(1, 1000) as usize))
    }

    pub fn by_type(&self, scan_id: ScanId) -> ApiResult<Vec<TypeGroup>> {
        Ok(self.scan(scan_id)?.tree.by_type())
    }

    pub fn access_denied(&self, scan_id: ScanId) -> ApiResult<Vec<AccessDeniedEntry>> {
        let scan = self.scan(scan_id)?;
        Ok(scan.access_denied.iter().map(|p| AccessDeniedEntry { path: p.clone() }).collect())
    }

    /// Runs both scanners on one folder. Blocking, so call it off the UI thread.
    pub fn compare_scanners(&self, path: &str) -> ApiResult<ScannerComparison> {
        fazasanj_scan::compare_scanners(Path::new(path), self.helper_path.clone(), Arc::new(AtomicBool::new(false)))
            .map_err(|e| ApiError::with_detail(e.code(), e.to_string()))
    }
}

/// (total, free) of the drive holding `path`, zeros when unknown.
#[cfg(windows)]
fn drive_space(path: &Path) -> (u64, u64) {
    fazasanj_platform::disk_space(path).map(|d| (d.total, d.free)).unwrap_or((0, 0))
}

#[cfg(not(windows))]
fn drive_space(_path: &Path) -> (u64, u64) {
    (0, 0)
}
