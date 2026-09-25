//! Fast scan client: starts the elevated helper, reads MFT records from a private pipe and
//! builds the same tree as the normal walker. Any failure becomes a fallback reason.

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

use fazasanj_model::{FallbackReason, ScanMode};
use fazasanj_platform::{
    filesystem_name, find_sidecar, launch_elevated, random_hex, ElevateError, ElevatedChild, PipeError, PipeServer,
    FAST_SCAN_HELPER,
};

use crate::ingest::{components_below_drive, Ingest};
use crate::options::{ScanOptions, ScanOutcome};
use crate::progress::{Counters, Reporter};
use crate::rootpath::{display_path, ScanRoot};
use crate::wire::{records, Frame, FrameDecoder, ERR_NOT_NTFS, VERSION};

/// How long the helper may take to connect once UAC was accepted.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
/// The helper sends progress every few hundred ms, so this much silence means it is stuck.
const IDLE_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub enum FastError {
    Cancelled,
    Fallback(FallbackReason, String),
}

fn helper_failed(detail: impl Into<String>) -> FastError {
    FastError::Fallback(FallbackReason::HelperFailed, detail.into())
}

fn pipe_err(e: PipeError, cancelled: bool) -> FastError {
    match e {
        _ if cancelled => FastError::Cancelled,
        PipeError::Timeout => FastError::Fallback(FallbackReason::Timeout, "pipe timed out".into()),
        other => helper_failed(other.to_string()),
    }
}

/// Checks the easy reasons not to try, before any UAC prompt.
fn preflight(opts: &ScanOptions, root: &ScanRoot) -> Result<(char, std::path::PathBuf), FastError> {
    let drive = root
        .drive
        .ok_or_else(|| FastError::Fallback(FallbackReason::NotNtfs, "not on a drive letter".into()))?;
    let fs = filesystem_name(std::path::Path::new(&root.display)).map_err(|e| helper_failed(e.to_string()))?;
    if !fs.eq_ignore_ascii_case("NTFS") {
        return Err(FastError::Fallback(FallbackReason::NotNtfs, fs));
    }
    let helper = opts
        .helper_path
        .clone()
        .or_else(|| find_sidecar(FAST_SCAN_HELPER))
        .ok_or_else(|| helper_failed("helper exe not found"))?;
    if !helper.is_file() {
        return Err(helper_failed(format!("helper exe not found at {}", helper.display())));
    }
    Ok((drive, helper))
}

pub fn run_fast(opts: &ScanOptions, root: &ScanRoot, started: Instant) -> Result<ScanOutcome, FastError> {
    let (drive, helper) = preflight(opts, root)?;
    let name = format!("fazasanj-{}", random_hex(16).map_err(|e| helper_failed(e.to_string()))?);
    let token = random_hex(32).map_err(|e| helper_failed(e.to_string()))?;
    // Admins are allowed too, for UAC with a different admin account. The token still has to match.
    let server = PipeServer::create(&name, true).map_err(|e| helper_failed(e.to_string()))?;

    let volume = format!("{drive}:");
    let args = ["--volume", volume.as_str(), "--pipe", name.as_str(), "--token", token.as_str()];
    let child = launch_elevated(&helper, &args).map_err(|e| match e {
        ElevateError::UacRefused => FastError::Fallback(FallbackReason::UacRefused, "uac refused".into()),
        other => helper_failed(other.to_string()),
    })?;

    let counters = Arc::new(Counters::default());
    counters.set_current(&root.display);
    let reporter = Reporter::start(counters.clone(), opts.progress.clone(), ScanMode::Fast, started);
    let result = receive(opts, root, &server, &child, &token, &counters);
    drop(server);
    let _ = child.wait(Some(Duration::from_secs(5)));
    match result {
        Ok(outcome) => {
            reporter.finish();
            Ok(ScanOutcome { duration_ms: started.elapsed().as_millis() as u64, ..outcome })
        }
        Err(e) => {
            drop(reporter);
            Err(e)
        }
    }
}

fn receive(
    opts: &ScanOptions,
    root: &ScanRoot,
    server: &PipeServer,
    child: &ElevatedChild,
    token: &str,
    counters: &Counters,
) -> Result<ScanOutcome, FastError> {
    let cancelled = || opts.cancel.load(Ordering::Relaxed);
    let gone = || cancelled() || matches!(child.exit_code(), Ok(Some(_)));
    server.wait_for_client(CONNECT_TIMEOUT, &gone).map_err(|e| match e {
        PipeError::Aborted if !cancelled() => helper_failed("helper exited before connecting"),
        other => pipe_err(other, cancelled()),
    })?;

    let mut decoder = FrameDecoder::default();
    let mut ingest = Ingest::new();
    let mut buf = vec![0u8; 1 << 20];
    let mut hello = false;
    loop {
        let n = server.read(&mut buf, IDLE_TIMEOUT, &cancelled).map_err(|e| pipe_err(e, cancelled()))?;
        if n == 0 {
            return Err(helper_failed("helper closed the pipe early"));
        }
        decoder.push(&buf[..n]);
        while let Some(frame) = decoder.next_frame().map_err(|e| helper_failed(e.to_string()))? {
            match frame {
                Frame::Hello { version, token: t } => {
                    if version != VERSION || t != token {
                        return Err(helper_failed("bad hello from helper"));
                    }
                    hello = true;
                }
                _ if !hello => return Err(helper_failed("helper did not say hello")),
                Frame::Records { count, payload } => {
                    let (mut files, mut dirs, mut bytes) = (0, 0, 0);
                    for r in records(count, &payload) {
                        let r = r.map_err(|e| helper_failed(e.to_string()))?;
                        let a = ingest.add(&r);
                        files += a.files;
                        dirs += a.dirs;
                        bytes += a.bytes;
                    }
                    counters.add(files, dirs, bytes);
                    if cancelled() {
                        return Err(FastError::Cancelled);
                    }
                }
                Frame::Progress { .. } => {}
                Frame::Error { code, message } => {
                    let reason = if code == ERR_NOT_NTFS { FallbackReason::NotNtfs } else { FallbackReason::HelperFailed };
                    return Err(FastError::Fallback(reason, message));
                }
                Frame::Done { .. } => return finish(opts, root, ingest),
            }
        }
    }
}

fn finish(opts: &ScanOptions, root: &ScanRoot, ingest: Ingest) -> Result<ScanOutcome, FastError> {
    let sub = components_below_drive(&root.display);
    let excluded: Vec<Vec<String>> = opts
        .excluded
        .iter()
        .map(|p| display_path(p))
        .filter(|p| p.get(..2).is_some_and(|d| root.display.get(..2).is_some_and(|r| r.eq_ignore_ascii_case(d))))
        .map(|p| components_below_drive(&p))
        .collect();
    let tree = ingest
        .finish(&sub, &excluded, root.display.clone())
        .ok_or_else(|| helper_failed("scan root not found in the MFT"))?;
    Ok(ScanOutcome {
        tree,
        scanner_used: ScanMode::Fast,
        fallback_reason: None,
        fallback_detail: None,
        duration_ms: 0,
        access_denied: Vec::new(),
    })
}
