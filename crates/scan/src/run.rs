//! `run_scan`: picks the scanner, handles fallback, reports progress.

use std::sync::Arc;
use std::time::Instant;

use fazasanj_model::{FallbackReason, ScanMode};

use crate::options::{ScanError, ScanOptions, ScanOutcome};
use crate::progress::{Counters, Reporter};
use crate::rootpath::{display_path, resolve_root, ScanRoot};
use crate::walker::walk;

/// Runs a scan on the calling thread and returns the finished tree.
///
/// Fast mode falls back to the normal scanner when the drive is not NTFS, UAC is refused,
/// or the helper fails. `ScanOutcome::fallback_reason` then says why.
pub fn run_scan(opts: ScanOptions) -> Result<ScanOutcome, ScanError> {
    let started = Instant::now();
    let root = resolve_root(&opts.root)?;
    match opts.mode {
        ScanMode::Normal => run_normal(&opts, &root, started, None),
        ScanMode::Fast => match crate::fast::run_fast(&opts, &root, started) {
            Ok(outcome) => Ok(outcome),
            Err(crate::fast::FastError::Cancelled) => Err(ScanError::Cancelled),
            Err(crate::fast::FastError::Fallback(reason, detail)) => {
                log::warn!("fast scan fell back to normal scan: {reason:?} ({detail})");
                run_normal(&opts, &root, started, Some((reason, detail)))
            }
        },
    }
}

pub(crate) fn excluded_keys(opts: &ScanOptions) -> Vec<String> {
    opts.excluded.iter().map(|p| display_path(p).to_lowercase()).collect()
}

pub(crate) fn run_normal(
    opts: &ScanOptions,
    root: &ScanRoot,
    started: Instant,
    fallback: Option<(FallbackReason, String)>,
) -> Result<ScanOutcome, ScanError> {
    let counters = Arc::new(Counters::default());
    let reporter = Reporter::start(counters.clone(), opts.progress.clone(), ScanMode::Normal, started);
    let excluded = excluded_keys(opts);
    let result = walk(root, &excluded, &opts.cancel, &counters);
    match result {
        Ok(out) => {
            reporter.finish();
            let (reason, detail) = fallback.map_or((None, None), |(r, d)| (Some(r), Some(d)));
            Ok(ScanOutcome {
                tree: out.tree,
                scanner_used: ScanMode::Normal,
                fallback_reason: reason,
                fallback_detail: detail,
                duration_ms: started.elapsed().as_millis() as u64,
                access_denied: out.access_denied,
            })
        }
        Err(e) => {
            drop(reporter);
            Err(e)
        }
    }
}
