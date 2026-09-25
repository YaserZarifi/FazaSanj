//! Public inputs and outputs of a scan.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use fazasanj_model::{FallbackReason, ScanMode};
use thiserror::Error;

use crate::tree::ScanTree;

/// A progress update. Sent about 10 times a second, plus once at the end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressSnapshot {
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
    pub current_path: String,
    pub elapsed_ms: u64,
    pub scanner: ScanMode,
}

pub type ProgressFn = Arc<dyn Fn(ProgressSnapshot) + Send + Sync>;

#[derive(Clone)]
pub struct ScanOptions {
    /// A drive root (`C:\`) or any folder.
    pub root: PathBuf,
    pub mode: ScanMode,
    /// Folders to leave out entirely (compared case insensitively).
    pub excluded: Vec<PathBuf>,
    /// Path to `fast-scan-helper.exe`. `None` looks next to the running exe.
    pub helper_path: Option<PathBuf>,
    /// Set to true to stop the scan. `run_scan` then returns `ScanError::Cancelled`.
    pub cancel: Arc<AtomicBool>,
    pub progress: ProgressFn,
}

impl ScanOptions {
    /// Options with no exclusions, no progress callback and a fresh cancel flag.
    pub fn new(root: impl Into<PathBuf>, mode: ScanMode) -> Self {
        Self {
            root: root.into(),
            mode,
            excluded: Vec::new(),
            helper_path: None,
            cancel: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(|_| {}),
        }
    }
}

impl std::fmt::Debug for ScanOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScanOptions")
            .field("root", &self.root)
            .field("mode", &self.mode)
            .field("excluded", &self.excluded)
            .field("helper_path", &self.helper_path)
            .finish()
    }
}

#[derive(Debug)]
pub struct ScanOutcome {
    pub tree: ScanTree,
    pub scanner_used: ScanMode,
    /// Set when fast mode was asked for but the normal scanner ran.
    pub fallback_reason: Option<FallbackReason>,
    /// Short technical detail about the fallback, for logs.
    pub fallback_detail: Option<String>,
    pub duration_ms: u64,
    /// Folders that could not be read.
    pub access_denied: Vec<String>,
}

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("scan cancelled")]
    Cancelled,
    #[error("folder not found: {0}")]
    RootNotFound(String),
    #[error("access denied: {0}")]
    RootAccessDenied(String),
    #[error("not a folder: {0}")]
    NotADirectory(String),
    #[error("fast scan unavailable: {0:?}")]
    FastUnavailable(FallbackReason),
    #[error("scan failed: {0}")]
    Io(String),
}

impl ScanError {
    /// Stable code for `ApiError`, translated by the UI as `errors.<code>`.
    pub fn code(&self) -> &'static str {
        match self {
            ScanError::Cancelled => "scan_cancelled",
            ScanError::RootNotFound(_) => "scan_root_not_found",
            ScanError::RootAccessDenied(_) => "scan_root_access_denied",
            ScanError::NotADirectory(_) => "scan_not_a_directory",
            ScanError::FastUnavailable(_) => "fast_scan_unavailable",
            ScanError::Io(_) => "scan_failed",
        }
    }
}
