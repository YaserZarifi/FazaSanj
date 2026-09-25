//! Cleanup history (P8). One row per run and one per action.

use std::collections::HashMap;

use fazasanj_model::{CleanupReport, HistoryEntry, LoggedAction};
use rusqlite::params;

use crate::convert::{enum_from_str, enum_to_str, to_i64, to_u64};
use crate::{Result, Store};

impl Store {
    /// Stores a finished (or dry) run and returns the new run id. The report's own `run_id`
    /// is ignored; the caller should copy the returned id into the report it sends to the UI.
    pub fn log_cleanup(&self, report: &CleanupReport) -> Result<i64> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO cleanup_runs (job_id, dry_run, free_before, free_after, bytes_freed,
                                       restore_point_created, started_at, finished_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                report.job_id,
                report.dry_run,
                to_i64(report.free_before),
                to_i64(report.free_after),
                to_i64(report.bytes_freed),
                report.restore_point_created,
                report.started_at,
                report.finished_at
            ],
        )?;
        let run_id = tx.last_insert_rowid();
        {
            let mut st = tx.prepare(
                "INSERT OR REPLACE INTO cleanup_actions
                   (run_id, idx, at, path, method, status, bytes, files_removed, files_skipped,
                    error_code, error_detail)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            )?;
            for a in &report.results {
                st.execute(params![
                    run_id,
                    a.index,
                    report.finished_at,
                    a.path,
                    enum_to_str(&a.method)?,
                    enum_to_str(&a.status)?,
                    to_i64(a.bytes_freed),
                    to_i64(a.files_removed),
                    to_i64(a.files_skipped),
                    a.error.as_ref().map(|e| e.code.as_str()),
                    a.error.as_ref().and_then(|e| e.detail.as_deref()),
                ])?;
            }
        }
        tx.commit()?;
        Ok(run_id)
    }

    /// Past runs, newest first. `restorable` is always false here; the app fills it in
    /// by checking the Recycle Bin.
    pub fn history(&self, limit: usize, offset: usize) -> Result<Vec<HistoryEntry>> {
        let conn = self.conn();
        let mut runs_st = conn.prepare(
            "SELECT id, started_at, finished_at, dry_run, bytes_freed FROM cleanup_runs
             ORDER BY started_at DESC, id DESC LIMIT ?1 OFFSET ?2",
        )?;
        let mut runs = runs_st
            .query_map(params![to_i64(limit as u64), to_i64(offset as u64)], |r| {
                Ok(HistoryEntry {
                    run_id: r.get(0)?,
                    started_at: r.get(1)?,
                    finished_at: r.get(2)?,
                    dry_run: r.get(3)?,
                    bytes_freed: to_u64(r.get(4)?),
                    actions: Vec::new(),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        if runs.is_empty() {
            return Ok(runs);
        }
        let index: HashMap<i64, usize> = runs
            .iter()
            .enumerate()
            .map(|(i, r)| (r.run_id, i))
            .collect();
        let (lo, hi) = runs.iter().fold((i64::MAX, i64::MIN), |(lo, hi), r| {
            (lo.min(r.run_id), hi.max(r.run_id))
        });
        let mut st = conn.prepare(
            "SELECT run_id, path, method, bytes, status, error_code FROM cleanup_actions
             WHERE run_id BETWEEN ?1 AND ?2 ORDER BY run_id, idx",
        )?;
        let mut rows = st.query(params![lo, hi])?;
        while let Some(r) = rows.next()? {
            let run_id: i64 = r.get(0)?;
            let Some(&i) = index.get(&run_id) else {
                continue;
            };
            let method: String = r.get(2)?;
            let status: String = r.get(4)?;
            runs[i].actions.push(LoggedAction {
                path: r.get(1)?,
                method: enum_from_str(&method)?,
                bytes: to_u64(r.get(3)?),
                status: enum_from_str(&status)?,
                restorable: false,
                error_code: r.get(5)?,
            });
        }
        Ok(runs)
    }

    pub fn history_count(&self) -> Result<u64> {
        let n: i64 = self
            .conn()
            .query_row("SELECT COUNT(*) FROM cleanup_runs", [], |r| r.get(0))?;
        Ok(to_u64(n))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fazasanj_model::{ActionResult, ActionStatus, ApiError, CleanupMethod};

    fn action(index: u32, path: &str, status: ActionStatus, freed: u64) -> ActionResult {
        ActionResult {
            index,
            path: path.into(),
            method: CleanupMethod::Recycle,
            status,
            bytes_freed: freed,
            files_removed: 3,
            files_skipped: 0,
            skipped_paths: vec![],
            error: None,
            would_remove: vec![],
        }
    }

    fn report(started_at: i64, results: Vec<ActionResult>) -> CleanupReport {
        CleanupReport {
            job_id: 7,
            run_id: 0,
            dry_run: false,
            free_before: 100,
            free_after: 200,
            bytes_freed: results.iter().map(|r| r.bytes_freed).sum(),
            restore_point_created: None,
            results,
            started_at,
            finished_at: started_at + 5,
        }
    }

    #[test]
    fn log_and_read_back() {
        let s = Store::open_in_memory().unwrap();
        let mut failed = action(1, "C:\\x\\locked", ActionStatus::Failed, 0);
        failed.method = CleanupMethod::DeleteContents;
        failed.error = Some(ApiError::with_detail("in_use", "held by app.exe"));
        let id1 = s
            .log_cleanup(&report(
                10,
                vec![action(0, "C:\\x\\cache", ActionStatus::Done, 50), failed],
            ))
            .unwrap();
        let id2 = s
            .log_cleanup(&report(20, vec![action(0, "C:\\y", ActionStatus::Done, 7)]))
            .unwrap();
        assert_ne!(id1, id2);

        let h = s.history(10, 0).unwrap();
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].run_id, id2);
        assert_eq!(h[1].run_id, id1);
        assert_eq!(h[1].bytes_freed, 50);
        assert_eq!(h[1].actions.len(), 2);
        let f = &h[1].actions[1];
        assert_eq!(f.method, CleanupMethod::DeleteContents);
        assert_eq!(f.status, ActionStatus::Failed);
        assert_eq!(f.error_code.as_deref(), Some("in_use"));
        assert!(!f.restorable);
        assert_eq!(s.history_count().unwrap(), 2);
    }

    #[test]
    fn history_paging() {
        let s = Store::open_in_memory().unwrap();
        for i in 0..5 {
            s.log_cleanup(&report(
                i,
                vec![action(0, &format!("C:\\p{i}"), ActionStatus::Done, 1)],
            ))
            .unwrap();
        }
        let page = s.history(2, 2).unwrap();
        assert_eq!(
            page.iter().map(|h| h.started_at).collect::<Vec<_>>(),
            vec![2, 1]
        );
        assert_eq!(page[0].actions[0].path, "C:\\p2");
        assert!(s.history(10, 10).unwrap().is_empty());
    }

    #[test]
    fn dry_run_flag_kept() {
        let s = Store::open_in_memory().unwrap();
        let mut r = report(1, vec![action(0, "C:\\a", ActionStatus::DryRun, 0)]);
        r.dry_run = true;
        s.log_cleanup(&r).unwrap();
        let h = s.history(1, 0).unwrap();
        assert!(h[0].dry_run);
        assert_eq!(h[0].actions[0].status, ActionStatus::DryRun);
    }
}
