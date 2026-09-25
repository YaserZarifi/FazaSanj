//! Last finished scan per root, so the app can say "last scanned 3 days ago" after a restart.

use fazasanj_model::ScanSummary;
use rusqlite::{params, OptionalExtension};

use crate::convert::path_key;
use crate::{Result, Store};

impl Store {
    pub fn set_last_scan(&self, summary: &ScanSummary) -> Result<()> {
        let raw = serde_json::to_string(summary)?;
        self.conn().execute(
            "INSERT INTO last_scans (root_key, root_path, finished_at, summary)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(root_key) DO UPDATE SET root_path = excluded.root_path,
                 finished_at = excluded.finished_at, summary = excluded.summary",
            params![path_key(&summary.root_path), summary.root_path, summary.finished_at, raw],
        )?;
        Ok(())
    }

    pub fn last_scan(&self, root: &str) -> Result<Option<ScanSummary>> {
        let raw: Option<String> = self
            .conn()
            .query_row(
                "SELECT summary FROM last_scans WHERE root_key = ?1",
                [path_key(root)],
                |r| r.get(0),
            )
            .optional()?;
        Ok(raw.and_then(|r| serde_json::from_str(&r).ok()))
    }

    /// Newest first. Rows that no longer parse are skipped.
    pub fn last_scans(&self) -> Result<Vec<ScanSummary>> {
        let conn = self.conn();
        let mut st = conn.prepare("SELECT summary FROM last_scans ORDER BY finished_at DESC")?;
        let raws = st
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(raws.iter().filter_map(|r| serde_json::from_str(r).ok()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fazasanj_model::ScanMode;

    fn summary(root: &str, finished_at: i64) -> ScanSummary {
        ScanSummary {
            scan_id: 1,
            root_path: root.into(),
            root_node: 0,
            total_bytes: 10,
            files: 1,
            dirs: 1,
            access_denied: 0,
            cloud_only: 0,
            duration_ms: 5,
            scanner: ScanMode::Normal,
            fallback_reason: None,
            drive_total: 100,
            drive_free: 90,
            finished_at,
        }
    }

    #[test]
    fn last_scan_per_root() {
        let s = Store::open_in_memory().unwrap();
        assert!(s.last_scan("C:\\").unwrap().is_none());
        s.set_last_scan(&summary("C:\\", 1)).unwrap();
        s.set_last_scan(&summary("D:\\", 3)).unwrap();
        s.set_last_scan(&summary("c:\\", 2)).unwrap();
        assert_eq!(s.last_scan("C:\\").unwrap().unwrap().finished_at, 2);
        let all = s.last_scans().unwrap();
        assert_eq!(all.iter().map(|x| x.finished_at).collect::<Vec<_>>(), vec![3, 2]);
    }
}
