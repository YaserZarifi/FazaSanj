//! Snapshots and growth over time.

use fazasanj_model::{GrowthPoint, ScanId, SnapshotComparison, SnapshotInfo};

use crate::{now_ms, store_err, ApiResult, App};

/// Changed folders listed in a comparison.
const COMPARE_LIMIT: usize = 300;
const GROWTH_POINTS: usize = 60;

impl App {
    pub fn list_snapshots(&self, root: Option<&str>) -> ApiResult<Vec<SnapshotInfo>> {
        self.store.list_snapshots(root).map_err(store_err)
    }

    pub fn compare_snapshots(&self, from_id: i64, to_id: i64) -> ApiResult<SnapshotComparison> {
        let mut cmp = self.store.compare(from_id, to_id, COMPARE_LIMIT).map_err(store_err)?;
        self.explain_growth(&mut cmp);
        Ok(cmp)
    }

    /// This scan compared with the previous snapshot of the same root, or None for a first scan.
    pub fn compare_with_last(&self, scan_id: ScanId) -> ApiResult<Option<SnapshotComparison>> {
        let scan = self.scan(scan_id)?;
        let Some(current) = scan.snapshot_id else {
            return Ok(None);
        };
        let prev = self.store.latest_before(&scan.summary.root_path, current).map_err(store_err)?;
        match prev {
            Some(prev) => self.compare_snapshots(prev.id, current).map(Some),
            None => Ok(None),
        }
    }

    pub fn growth(&self, path: &str) -> ApiResult<Vec<GrowthPoint>> {
        self.store.growth(path, GROWTH_POINTS).map_err(store_err)
    }

    fn explain_growth(&self, cmp: &mut SnapshotComparison) {
        let now = now_ms();
        for item in &mut cmp.items {
            item.explanation =
                self.rules.match_path(&item.path, true, None, now).and_then(|i| self.rules.explanation(i));
        }
    }
}
