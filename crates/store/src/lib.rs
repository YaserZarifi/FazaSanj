//! SQLite storage: snapshots, cleanup log, AI cache, settings.
//!
//! One connection behind a mutex. Every call is short, so a pool would buy nothing here and
//! would complicate in-memory databases (each pooled connection would see its own empty db).

mod ai_cache;
mod cleanup_log;
mod convert;
mod error;
mod kv;
mod migrations;
mod scans;
mod snapshots;

use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;

pub use error::StoreError;
pub use snapshots::SnapshotRow;

pub type Result<T> = std::result::Result<T, StoreError>;

pub struct Store {
    conn: Mutex<Connection>,
}

impl std::fmt::Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Store").finish_non_exhaustive()
    }
}

impl Store {
    /// Opens (or creates) the database file and runs pending migrations.
    pub fn open(path: &Path) -> Result<Store> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        Self::init(conn)
    }

    pub fn open_in_memory() -> Result<Store> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> Result<Store> {
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        migrations::run(&mut conn)?;
        Ok(Store {
            conn: Mutex::new(conn),
        })
    }

    /// A panic while holding the lock cannot leave SQLite half-written (transactions roll back
    /// on drop), so a poisoned lock is safe to reuse.
    fn conn(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Current schema version (PRAGMA user_version).
    pub fn schema_version(&self) -> Result<i64> {
        Ok(self
            .conn()
            .pragma_query_value(None, "user_version", |r| r.get(0))?)
    }

    /// Removes every stored row (settings, snapshots, history, cache, scan metadata).
    pub fn wipe(&self) -> Result<()> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        for table in migrations::DATA_TABLES {
            tx.execute(&format!("DELETE FROM {table}"), [])?;
        }
        tx.commit()?;
        // Give the space back; harmless if it fails (e.g. another reader holds the file).
        let _ = conn.execute_batch("VACUUM");
        Ok(())
    }

    /// Alias for [`Store::wipe`], matching the "reset everything" button in settings.
    pub fn reset_everything(&self) -> Result<()> {
        self.wipe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn store_is_send_sync() {
        assert_send_sync::<Store>();
    }

    #[test]
    fn migrations_set_version() {
        let s = Store::open_in_memory().unwrap();
        assert_eq!(s.schema_version().unwrap(), migrations::latest_version());
    }

    #[test]
    fn reopen_file_keeps_data_and_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub").join("fazasanj.db");
        {
            let s = Store::open(&path).unwrap();
            s.set_value("k", "v").unwrap();
        }
        let s = Store::open(&path).unwrap();
        assert_eq!(s.get_value("k").unwrap().as_deref(), Some("v"));
        assert_eq!(s.schema_version().unwrap(), migrations::latest_version());
        let mode: String = s
            .conn()
            .pragma_query_value(None, "journal_mode", |r| r.get(0))
            .unwrap();
        assert_eq!(mode.to_lowercase(), "wal");
    }

    #[test]
    fn wipe_clears_everything() {
        let s = Store::open_in_memory().unwrap();
        s.set_value("k", "v").unwrap();
        s.save_snapshot("C:\\", 1, 10, 1, 100, 50, &[("C:\\".into(), 10, 1)])
            .unwrap();
        s.wipe().unwrap();
        assert!(s.get_value("k").unwrap().is_none());
        assert!(s.list_snapshots(None).unwrap().is_empty());
    }

    #[test]
    fn shared_between_threads() {
        let s = std::sync::Arc::new(Store::open_in_memory().unwrap());
        let handles: Vec<_> = (0..4)
            .map(|i| {
                let s = s.clone();
                std::thread::spawn(move || {
                    for j in 0..50 {
                        s.set_value(&format!("t{i}-{j}"), "x").unwrap();
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(s.get_value("t3-49").unwrap().as_deref(), Some("x"));
    }
}
