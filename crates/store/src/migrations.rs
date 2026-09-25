//! Versioned schema. Each entry runs once, in order, and bumps PRAGMA user_version.
//! Never edit an entry that has shipped; add a new one.

use rusqlite::Connection;

use crate::Result;

const MIGRATIONS: &[&str] = &[
    // 1: initial schema
    r#"
    CREATE TABLE kv (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );

    CREATE TABLE paths (
        id      INTEGER PRIMARY KEY,
        key     TEXT NOT NULL UNIQUE,
        display TEXT NOT NULL
    );

    CREATE TABLE snapshots (
        id          INTEGER PRIMARY KEY,
        root_path   TEXT NOT NULL,
        root_key    TEXT NOT NULL,
        taken_at    INTEGER NOT NULL,
        total_bytes INTEGER NOT NULL,
        files       INTEGER NOT NULL,
        drive_total INTEGER NOT NULL,
        drive_free  INTEGER NOT NULL
    );
    CREATE INDEX snapshots_root_time ON snapshots(root_key, taken_at);

    CREATE TABLE snapshot_rows (
        snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
        path_id     INTEGER NOT NULL REFERENCES paths(id),
        size        INTEGER NOT NULL,
        files       INTEGER NOT NULL,
        PRIMARY KEY (snapshot_id, path_id)
    ) WITHOUT ROWID;
    CREATE INDEX snapshot_rows_path ON snapshot_rows(path_id);

    CREATE TABLE cleanup_runs (
        id                    INTEGER PRIMARY KEY,
        job_id                INTEGER NOT NULL,
        dry_run               INTEGER NOT NULL,
        free_before           INTEGER NOT NULL,
        free_after            INTEGER NOT NULL,
        bytes_freed           INTEGER NOT NULL,
        restore_point_created INTEGER,
        started_at            INTEGER NOT NULL,
        finished_at           INTEGER NOT NULL
    );
    CREATE INDEX cleanup_runs_time ON cleanup_runs(started_at);

    CREATE TABLE cleanup_actions (
        run_id        INTEGER NOT NULL REFERENCES cleanup_runs(id) ON DELETE CASCADE,
        idx           INTEGER NOT NULL,
        at            INTEGER NOT NULL,
        path          TEXT NOT NULL,
        method        TEXT NOT NULL,
        status        TEXT NOT NULL,
        bytes         INTEGER NOT NULL,
        files_removed INTEGER NOT NULL,
        files_skipped INTEGER NOT NULL,
        error_code    TEXT,
        error_detail  TEXT,
        PRIMARY KEY (run_id, idx)
    ) WITHOUT ROWID;

    CREATE TABLE ai_cache (
        key        TEXT PRIMARY KEY,
        answer     TEXT NOT NULL,
        created_at INTEGER NOT NULL
    );

    CREATE TABLE last_scans (
        root_key    TEXT PRIMARY KEY,
        root_path   TEXT NOT NULL,
        finished_at INTEGER NOT NULL,
        summary     TEXT NOT NULL
    );
    "#,
];

/// Tables holding user data, children before parents so deletes respect foreign keys.
pub(crate) const DATA_TABLES: &[&str] = &[
    "snapshot_rows",
    "snapshots",
    "paths",
    "cleanup_actions",
    "cleanup_runs",
    "ai_cache",
    "last_scans",
    "kv",
];

#[cfg(test)]
pub(crate) fn latest_version() -> i64 {
    MIGRATIONS.len() as i64
}

pub(crate) fn run(conn: &mut Connection) -> Result<()> {
    let current: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    for (i, sql) in MIGRATIONS.iter().enumerate() {
        let version = i as i64 + 1;
        if version <= current {
            continue;
        }
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", version)?;
        tx.commit()?;
    }
    Ok(())
}
