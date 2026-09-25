//! MFT records to wire frames.

use std::cell::RefCell;
use std::io::Write;
use std::time::{Duration, Instant};

use crate::merge::Merger;
use crate::mft::{read_mft, MftLayout, VolumeRead};
use crate::record::parse_record;
use crate::wire::{encode_progress, RecordBatch, WireRecord};

const PROGRESS_EVERY: Duration = Duration::from_millis(250);

#[derive(Debug)]
pub enum StreamError {
    /// The app went away (cancelled or crashed). Nothing more to do.
    PipeClosed,
    Read(String),
}

struct Sink<'a, W: Write> {
    out: &'a mut W,
    batch: RecordBatch,
    frame: Vec<u8>,
    sent: u64,
    failed: bool,
}

impl<W: Write> Sink<'_, W> {
    fn push(&mut self, r: &WireRecord) {
        self.batch.push(r);
        self.sent += 1;
        if self.batch.is_full() {
            self.flush_batch();
        }
    }

    fn flush_batch(&mut self) {
        self.batch.flush_into(&mut self.frame);
        self.write_frame();
    }

    fn progress(&mut self, done: u64, total: u64) {
        // Keeps the app's idle timer happy even when a stretch of the MFT has no live records.
        self.flush_batch();
        encode_progress(&mut self.frame, done, total);
        self.write_frame();
        if !self.failed && self.out.flush().is_err() {
            self.failed = true;
        }
    }

    fn write_frame(&mut self) {
        if !self.failed && !self.frame.is_empty() && self.out.write_all(&self.frame).is_err() {
            self.failed = true;
        }
        self.frame.clear();
    }
}

/// Reads every record and writes `RECORDS` and `PROGRESS` frames. Returns how many wire
/// records were sent. The caller writes `HELLO` before and `DONE` after.
pub fn stream_mft<W: Write>(vol: &impl VolumeRead, layout: &MftLayout, out: &mut W) -> Result<u64, StreamError> {
    let sink = RefCell::new(Sink { out, batch: RecordBatch::default(), frame: Vec::new(), sent: 0, failed: false });
    let mut merger = Merger::default();
    let mut last = Instant::now();

    let result = read_mft(
        vol,
        layout,
        &mut |done, total| {
            let mut s = sink.borrow_mut();
            if last.elapsed() >= PROGRESS_EVERY {
                last = Instant::now();
                s.progress(done, total);
            }
            // Stop reading as soon as nobody is listening.
            !s.failed
        },
        &mut |n, bytes| {
            // Unused, zeroed or torn records are skipped.
            if let Ok(p) = parse_record(bytes) {
                merger.add(n, p, &mut |r: &WireRecord| sink.borrow_mut().push(r));
            }
        },
    );
    merger.finish(&mut |r: &WireRecord| sink.borrow_mut().push(r));
    let mut s = sink.into_inner();
    s.flush_batch();
    if s.failed {
        return Err(StreamError::PipeClosed);
    }
    result.map_err(StreamError::Read)?;
    Ok(s.sent)
}
