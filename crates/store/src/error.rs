use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("could not encode or decode stored json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("could not create the data folder: {0}")]
    Io(#[from] std::io::Error),
    #[error("snapshot {0} not found")]
    SnapshotNotFound(i64),
    #[error("stored value is not valid: {0}")]
    BadValue(String),
}

impl StoreError {
    /// Stable code for `ApiError`, translated by the UI.
    pub fn code(&self) -> &'static str {
        match self {
            StoreError::SnapshotNotFound(_) => "snapshot_not_found",
            _ => "storage_error",
        }
    }
}
