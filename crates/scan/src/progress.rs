//! Shared scan counters and a reporter thread that calls the progress callback ~10 times a second.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use fazasanj_model::ScanMode;

use crate::options::{ProgressFn, ProgressSnapshot};

const TICK: Duration = Duration::from_millis(100);

#[derive(Default)]
pub struct Counters {
    pub files: AtomicU64,
    pub dirs: AtomicU64,
    pub bytes: AtomicU64,
    current: Mutex<String>,
}

impl Counters {
    pub fn add(&self, files: u64, dirs: u64, bytes: u64) {
        self.files.fetch_add(files, Ordering::Relaxed);
        self.dirs.fetch_add(dirs, Ordering::Relaxed);
        self.bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Updates the current path unless another thread holds the lock (not worth waiting for).
    pub fn set_current(&self, path: &str) {
        if let Ok(mut c) = self.current.try_lock() {
            c.clear();
            c.push_str(path);
        }
    }

    fn snapshot(&self, started: Instant, scanner: ScanMode) -> ProgressSnapshot {
        ProgressSnapshot {
            files: self.files.load(Ordering::Relaxed),
            dirs: self.dirs.load(Ordering::Relaxed),
            bytes: self.bytes.load(Ordering::Relaxed),
            current_path: self.current.lock().map(|c| c.clone()).unwrap_or_default(),
            elapsed_ms: started.elapsed().as_millis() as u64,
            scanner,
        }
    }
}

/// Runs the callback on its own thread until dropped or `finish` is called.
pub struct Reporter {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    counters: Arc<Counters>,
    callback: ProgressFn,
    started: Instant,
    scanner: ScanMode,
}

impl Reporter {
    pub fn start(counters: Arc<Counters>, callback: ProgressFn, scanner: ScanMode, started: Instant) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let (s, c, cb) = (stop.clone(), counters.clone(), callback.clone());
        let handle = std::thread::Builder::new()
            .name("scan-progress".into())
            .spawn(move || {
                while !s.load(Ordering::Relaxed) {
                    std::thread::park_timeout(TICK);
                    if s.load(Ordering::Relaxed) {
                        break;
                    }
                    cb(c.snapshot(started, scanner));
                }
            })
            .ok();
        Self { stop, handle, counters, callback, started, scanner }
    }

    /// Stops the thread and sends one last snapshot.
    pub fn finish(mut self) {
        self.shutdown();
        (self.callback)(self.counters.snapshot(self.started, self.scanner));
    }

    fn shutdown(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            h.thread().unpark();
            let _ = h.join();
        }
    }
}

impl Drop for Reporter {
    fn drop(&mut self) {
        self.shutdown();
    }
}
