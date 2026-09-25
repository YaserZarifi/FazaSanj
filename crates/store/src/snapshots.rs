//! Compact folder-size snapshots (P10). Paths are interned in `paths` so a folder that shows
//! up in every weekly snapshot is stored once.

use fazasanj_model::{GrowthItem, GrowthPoint, SnapshotComparison, SnapshotInfo};
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::convert::{path_key, to_i64, to_u64};
use crate::{Result, Store, StoreError};

/// (folder path, size in bytes, file count)
pub type SnapshotRow = (String, u64, u64);

const INFO_COLS: &str = "id, root_path, taken_at, total_bytes, files, drive_total, drive_free";

const DROP_UNUSED_PATHS: &str = "DELETE FROM paths WHERE NOT EXISTS (
     SELECT 1 FROM snapshot_rows r WHERE r.path_id = paths.id
 )";

fn info_from_row(r: &Row<'_>) -> rusqlite::Result<SnapshotInfo> {
    Ok(SnapshotInfo {
        id: r.get(0)?,
        root_path: r.get(1)?,
        taken_at: r.get(2)?,
        total_bytes: to_u64(r.get(3)?),
        files: to_u64(r.get(4)?),
        drive_total: to_u64(r.get(5)?),
        drive_free: to_u64(r.get(6)?),
    })
}

fn get_info(conn: &Connection, id: i64) -> Result<SnapshotInfo> {
    conn.query_row(
        &format!("SELECT {INFO_COLS} FROM snapshots WHERE id = ?1"),
        [id],
        info_from_row,
    )
    .optional()?
    .ok_or(StoreError::SnapshotNotFound(id))
}

fn delta(before: u64, after: u64) -> i64 {
    to_i64(after).saturating_sub(to_i64(before))
}

impl Store {
    /// Saves one snapshot in a single transaction and returns its id.
    #[allow(clippy::too_many_arguments)]
    pub fn save_snapshot(
        &self,
        root_path: &str,
        taken_at: i64,
        total_bytes: u64,
        files: u64,
        drive_total: u64,
        drive_free: u64,
        rows: &[SnapshotRow],
    ) -> Result<i64> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO snapshots
               (root_path, root_key, taken_at, total_bytes, files, drive_total, drive_free)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                root_path,
                path_key(root_path),
                taken_at,
                to_i64(total_bytes),
                to_i64(files),
                to_i64(drive_total),
                to_i64(drive_free)
            ],
        )?;
        let snapshot_id = tx.last_insert_rowid();
        {
            // The no-op update makes RETURNING give the id for existing paths too.
            let mut intern = tx.prepare(
                "INSERT INTO paths (key, display) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET key = key
                 RETURNING id",
            )?;
            let mut insert = tx.prepare(
                "INSERT OR REPLACE INTO snapshot_rows (snapshot_id, path_id, size, files)
                 VALUES (?1, ?2, ?3, ?4)",
            )?;
            for (path, size, count) in rows {
                let path_id: i64 = intern.query_row(params![path_key(path), path], |r| r.get(0))?;
                insert.execute(params![snapshot_id, path_id, to_i64(*size), to_i64(*count)])?;
            }
        }
        tx.commit()?;
        Ok(snapshot_id)
    }

    /// Newest first. `root` filters by root path (case-insensitive).
    pub fn list_snapshots(&self, root: Option<&str>) -> Result<Vec<SnapshotInfo>> {
        let conn = self.conn();
        let list = match root {
            Some(root) => {
                let mut st = conn.prepare(&format!(
                    "SELECT {INFO_COLS} FROM snapshots WHERE root_key = ?1
                     ORDER BY taken_at DESC, id DESC"
                ))?;
                let rows = st.query_map([path_key(root)], info_from_row)?;
                rows.collect::<rusqlite::Result<Vec<_>>>()?
            }
            None => {
                let mut st = conn.prepare(&format!(
                    "SELECT {INFO_COLS} FROM snapshots ORDER BY taken_at DESC, id DESC"
                ))?;
                let rows = st.query_map([], info_from_row)?;
                rows.collect::<rusqlite::Result<Vec<_>>>()?
            }
        };
        Ok(list)
    }

    pub fn get_snapshot(&self, id: i64) -> Result<SnapshotInfo> {
        get_info(&self.conn(), id)
    }

    /// The snapshot of the same root taken just before snapshot `id`.
    pub fn latest_before(&self, root: &str, id: i64) -> Result<Option<SnapshotInfo>> {
        let conn = self.conn();
        let current = get_info(&conn, id)?;
        let prev = conn
            .query_row(
                &format!(
                    "SELECT {INFO_COLS} FROM snapshots
                     WHERE root_key = ?1 AND id <> ?2
                       AND (taken_at < ?3 OR (taken_at = ?3 AND id < ?2))
                     ORDER BY taken_at DESC, id DESC LIMIT 1"
                ),
                params![path_key(root), id, current.taken_at],
                info_from_row,
            )
            .optional()?;
        Ok(prev)
    }

    /// Folders that changed between two snapshots, biggest absolute change first.
    /// A folder missing from one side counts as 0 there. `explanation` is left for the app.
    pub fn compare(&self, from_id: i64, to_id: i64, limit: usize) -> Result<SnapshotComparison> {
        let conn = self.conn();
        let from = get_info(&conn, from_id)?;
        let to = get_info(&conn, to_id)?;
        let mut st = conn.prepare(
            "SELECT p.display, d.before, d.after FROM (
                 SELECT path_id,
                        SUM(CASE WHEN snapshot_id = ?1 THEN size ELSE 0 END) AS before,
                        SUM(CASE WHEN snapshot_id = ?2 THEN size ELSE 0 END) AS after
                 FROM snapshot_rows
                 WHERE snapshot_id IN (?1, ?2)
                 GROUP BY path_id
             ) d
             JOIN paths p ON p.id = d.path_id
             WHERE d.before <> d.after
             ORDER BY ABS(d.after - d.before) DESC, p.key
             LIMIT ?3",
        )?;
        let items = st
            .query_map(params![from_id, to_id, to_i64(limit as u64)], |r| {
                let before = to_u64(r.get(1)?);
                let after = to_u64(r.get(2)?);
                Ok(GrowthItem {
                    path: r.get(0)?,
                    before,
                    after,
                    delta: delta(before, after),
                    explanation: None,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let total_delta = delta(from.total_bytes, to.total_bytes);
        Ok(SnapshotComparison { from, to, total_delta, items })
    }

    /// Size history of one folder across all snapshots that contain it, oldest first.
    /// Keeps the newest `limit` points.
    pub fn growth(&self, path: &str, limit: usize) -> Result<Vec<GrowthPoint>> {
        let conn = self.conn();
        let mut st = conn.prepare(
            "SELECT taken_at, size FROM (
                 SELECT s.taken_at, s.id, r.size
                 FROM paths p
                 JOIN snapshot_rows r ON r.path_id = p.id
                 JOIN snapshots s ON s.id = r.snapshot_id
                 WHERE p.key = ?1
                 ORDER BY s.taken_at DESC, s.id DESC
                 LIMIT ?2
             ) ORDER BY taken_at, id",
        )?;
        let points = st
            .query_map(params![path_key(path), to_i64(limit as u64)], |r| {
                Ok(GrowthPoint { taken_at: r.get(0)?, bytes: to_u64(r.get(1)?) })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(points)
    }

    /// Keeps the newest `keep_per_root` snapshots of each root and deletes the rest, along with
    /// paths no snapshot uses any more. Returns how many snapshots were removed.
    pub fn prune_snapshots(&self, keep_per_root: usize) -> Result<usize> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        let removed = tx.execute(
            "DELETE FROM snapshots WHERE id IN (
                 SELECT id FROM (
                     SELECT id, ROW_NUMBER() OVER (
                         PARTITION BY root_key ORDER BY taken_at DESC, id DESC
                     ) AS n
                     FROM snapshots
                 ) WHERE n > ?1
             )",
            [to_i64(keep_per_root as u64)],
        )?;
        if removed > 0 {
            tx.execute(DROP_UNUSED_PATHS, [])?;
        }
        tx.commit()?;
        Ok(removed)
    }

    pub fn delete_snapshot(&self, id: i64) -> Result<()> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM snapshots WHERE id = ?1", [id])?;
        tx.execute(DROP_UNUSED_PATHS, [])?;
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(items: &[(&str, u64)]) -> Vec<SnapshotRow> {
        items.iter().map(|(p, s)| (p.to_string(), *s, 1)).collect()
    }

    fn count(s: &Store, table: &str) -> i64 {
        s.conn().query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0)).unwrap()
    }

    #[test]
    fn save_and_list() {
        let s = Store::open_in_memory().unwrap();
        let a = s.save_snapshot("C:\\", 100, 10, 2, 1000, 500, &rows(&[("C:\\", 10)])).unwrap();
        let b = s.save_snapshot("c:\\", 200, 20, 3, 1000, 490, &rows(&[("C:\\", 20)])).unwrap();
        let d = s.save_snapshot("D:\\", 150, 5, 1, 900, 800, &rows(&[("D:\\", 5)])).unwrap();
        let all = s.list_snapshots(None).unwrap();
        assert_eq!(all.iter().map(|i| i.id).collect::<Vec<_>>(), vec![b, d, a]);
        let c = s.list_snapshots(Some("C:\\")).unwrap();
        assert_eq!(c.iter().map(|i| i.id).collect::<Vec<_>>(), vec![b, a]);
        assert_eq!(c[0].drive_free, 490);
        assert_eq!(s.get_snapshot(a).unwrap().root_path, "C:\\");
        assert!(matches!(s.get_snapshot(999), Err(StoreError::SnapshotNotFound(999))));
    }

    #[test]
    fn paths_are_interned() {
        let s = Store::open_in_memory().unwrap();
        let r = rows(&[("C:\\", 10), ("C:\\Users", 5), ("C:\\Windows", 4)]);
        s.save_snapshot("C:\\", 1, 10, 1, 0, 0, &r).unwrap();
        s.save_snapshot("C:\\", 2, 10, 1, 0, 0, &r).unwrap();
        // Different case is the same folder on Windows.
        s.save_snapshot("C:\\", 3, 10, 1, 0, 0, &rows(&[("c:\\users\\", 6)])).unwrap();
        assert_eq!(count(&s, "paths"), 3);
        assert_eq!(count(&s, "snapshot_rows"), 7);
    }

    #[test]
    fn latest_before_finds_previous_of_same_root() {
        let s = Store::open_in_memory().unwrap();
        let a = s.save_snapshot("C:\\", 100, 1, 1, 0, 0, &[]).unwrap();
        s.save_snapshot("D:\\", 150, 1, 1, 0, 0, &[]).unwrap();
        let b = s.save_snapshot("C:\\", 200, 1, 1, 0, 0, &[]).unwrap();
        assert_eq!(s.latest_before("C:\\", b).unwrap().map(|i| i.id), Some(a));
        assert!(s.latest_before("C:\\", a).unwrap().is_none());
    }

    #[test]
    fn compare_sorts_by_abs_delta() {
        let s = Store::open_in_memory().unwrap();
        let before = rows(&[("C:\\", 100), ("C:\\A", 50), ("C:\\B", 30), ("C:\\Gone", 20)]);
        let after = rows(&[("C:\\", 150), ("C:\\A", 50), ("C:\\B", 100), ("C:\\New", 5)]);
        let a = s.save_snapshot("C:\\", 1, 100, 1, 0, 0, &before).unwrap();
        let b = s.save_snapshot("C:\\", 2, 150, 1, 0, 0, &after).unwrap();
        let cmp = s.compare(a, b, 10).unwrap();
        assert_eq!(cmp.total_delta, 50);
        let got: Vec<_> = cmp.items.iter().map(|i| (i.path.as_str(), i.delta)).collect();
        assert_eq!(got, vec![("C:\\B", 70), ("C:\\", 50), ("C:\\Gone", -20), ("C:\\New", 5)]);
        assert_eq!(cmp.items[2].after, 0);
        assert!(cmp.items.iter().all(|i| i.explanation.is_none()));
        assert_eq!(s.compare(a, b, 2).unwrap().items.len(), 2);
        assert!(s.compare(a, 999, 10).is_err());
    }

    #[test]
    fn growth_is_case_insensitive_and_limited() {
        let s = Store::open_in_memory().unwrap();
        for i in 0..5u64 {
            let r = rows(&[("C:\\Users\\Ali", i * 100)]);
            s.save_snapshot("C:\\", i as i64 * 10, 0, 0, 0, 0, &r).unwrap();
        }
        let g = s.growth("c:\\users\\ALI\\", 3).unwrap();
        assert_eq!(
            g,
            vec![
                GrowthPoint { taken_at: 20, bytes: 200 },
                GrowthPoint { taken_at: 30, bytes: 300 },
                GrowthPoint { taken_at: 40, bytes: 400 },
            ]
        );
        assert!(s.growth("C:\\Nope", 3).unwrap().is_empty());
    }

    #[test]
    fn prune_keeps_newest_per_root() {
        let s = Store::open_in_memory().unwrap();
        for i in 0..4 {
            let only = format!("C:\\only{i}");
            let r = rows(&[("C:\\", 1), (only.as_str(), 1)]);
            s.save_snapshot("C:\\", i, 0, 0, 0, 0, &r).unwrap();
        }
        s.save_snapshot("D:\\", 1, 0, 0, 0, 0, &rows(&[("D:\\", 1)])).unwrap();
        assert_eq!(s.prune_snapshots(2).unwrap(), 2);
        let c = s.list_snapshots(Some("C:\\")).unwrap();
        assert_eq!(c.iter().map(|i| i.taken_at).collect::<Vec<_>>(), vec![3, 2]);
        assert_eq!(s.list_snapshots(Some("D:\\")).unwrap().len(), 1);
        // C:\, only2, only3, D:\
        assert_eq!(count(&s, "paths"), 4);
        assert_eq!(count(&s, "snapshot_rows"), 5);
    }

    #[test]
    fn delete_snapshot_cascades() {
        let s = Store::open_in_memory().unwrap();
        let a = s.save_snapshot("C:\\", 1, 0, 0, 0, 0, &rows(&[("C:\\x", 1)])).unwrap();
        s.delete_snapshot(a).unwrap();
        assert_eq!(count(&s, "snapshot_rows"), 0);
        assert_eq!(count(&s, "paths"), 0);
    }

    #[test]
    fn big_snapshot_is_fast() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::open(&dir.path().join("perf.db")).unwrap();
        let rows: Vec<SnapshotRow> = (0..50_000u64)
            .map(|i| (format!("C:\\Users\\user\\folder{}\\sub{}", i / 100, i), i * 1024, i))
            .collect();
        let t = std::time::Instant::now();
        let first = s.save_snapshot("C:\\", 1, 1, 1, 1, 1, &rows).unwrap();
        let first_ms = t.elapsed().as_millis();
        let t = std::time::Instant::now();
        let second = s.save_snapshot("C:\\", 2, 1, 1, 1, 1, &rows).unwrap();
        let second_ms = t.elapsed().as_millis();
        let t = std::time::Instant::now();
        let cmp = s.compare(first, second, 100).unwrap();
        let cmp_ms = t.elapsed().as_millis();
        assert!(cmp.items.is_empty());
        println!("50k rows: first save {first_ms} ms, second save {second_ms} ms, compare {cmp_ms} ms");
        // Loose bound so slow CI machines pass; the release target is under 1s.
        assert!(first_ms < 10_000);
    }
}
