//! fast-scan-helper: runs elevated, reads the NTFS MFT of one volume and streams file records
//! to the Fazasanj app over a named pipe. Read only. It never writes to or deletes anything
//! on the volume; the only thing it writes to is the app's pipe.
//!
//! Usage: fast-scan-helper --volume C: --pipe fazasanj-<hex> --token <hex>
//!
//! Exit codes: 0 ok, 2 bad arguments, 3 could not reach the app, 4 scan failed (error sent).

mod args;
mod merge;
mod mft;
mod record;
mod runlist;
mod stream;
// Shared with the app, some items are only used on the app side.
#[allow(dead_code)]
#[path = "../../scan/src/wire.rs"]
mod wire;

use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Duration;

use fazasanj_platform::{connect_pipe, file_extents, filesystem_name, RawVolume};

use crate::mft::{MftLayout, VolumeRead};
use crate::stream::{stream_mft, StreamError};

struct Volume(RawVolume);

impl VolumeRead for Volume {
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, String> {
        self.0.read_at(offset, buf).map_err(|e| e.to_string())
    }
}

fn layout(vol: &RawVolume, letter: char) -> Result<MftLayout, (u16, String)> {
    let d = vol.ntfs_data().map_err(|e| (wire::ERR_OPEN_VOLUME, e.to_string()))?;
    let cluster = u64::from(d.bytes_per_cluster);
    let record = d.bytes_per_record as usize;
    if cluster == 0 || !(512..=65536).contains(&record) {
        return Err((wire::ERR_READ, "odd volume geometry".into()));
    }
    let mft_path = format!(r"{letter}:\$MFT");
    let extents = match file_extents(Path::new(&mft_path)) {
        Ok(e) if !e.is_empty() => e,
        _ => {
            // Fall back to the run list stored in record 0 of the MFT itself.
            let mut rec0 = vec![0u8; record.max(4096).next_multiple_of(4096)];
            vol.read_at(d.mft_start_lcn * cluster, &mut rec0).map_err(|e| (wire::ERR_READ, e.to_string()))?;
            runlist::mft_extents_from_record(&mut rec0[..record]).map_err(|e| (wire::ERR_READ, format!("{e:?}")))?
        }
    };
    Ok(MftLayout { cluster_size: cluster, record_size: record, valid_len: d.mft_valid_data_length, extents })
}

fn scan(letter: char, out: &mut impl Write) -> Result<u64, (u16, String)> {
    let root = format!(r"{letter}:\");
    let fs = filesystem_name(Path::new(&root)).map_err(|e| (wire::ERR_OPEN_VOLUME, e.to_string()))?;
    if !fs.eq_ignore_ascii_case("NTFS") {
        return Err((wire::ERR_NOT_NTFS, fs));
    }
    let vol = RawVolume::open(letter).map_err(|e| (wire::ERR_OPEN_VOLUME, e.to_string()))?;
    let layout = layout(&vol, letter)?;
    stream_mft(&Volume(vol), &layout, out).map_err(|e| match e {
        StreamError::PipeClosed => (0, String::new()),
        StreamError::Read(m) => (wire::ERR_READ, m),
    })
}

fn run() -> i32 {
    let args = match args::parse(std::env::args().skip(1)) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("fast-scan-helper: {e}");
            return 2;
        }
    };
    let Ok(pipe) = connect_pipe(&args.pipe, Duration::from_secs(10)) else { return 3 };
    let mut out = BufWriter::with_capacity(1 << 20, pipe);
    let mut frame = Vec::new();
    wire::encode_hello(&mut frame, &args.token);
    if out.write_all(&frame).and_then(|_| out.flush()).is_err() {
        return 3;
    }
    frame.clear();
    let code = match scan(args.letter, &mut out) {
        Ok(n) => {
            wire::encode_done(&mut frame, n);
            0
        }
        Err((0, _)) => return 3,
        Err((code, msg)) => {
            wire::encode_error(&mut frame, code, &msg);
            4
        }
    };
    if out.write_all(&frame).and_then(|_| out.flush()).is_err() {
        return 3;
    }
    code
}

fn main() {
    std::process::exit(run());
}

#[cfg(test)]
mod tests;
