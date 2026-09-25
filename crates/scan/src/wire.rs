//! Wire format between `fast-scan-helper` and the app.
//!
//! Standard library only: the helper includes this exact file with `#[path]`, so both sides
//! always agree without the helper depending on the scan crate.
//!
//! Stream: a sequence of frames. Every frame is `kind: u8`, `len: u32 LE`, then `len` bytes.
//! The helper sends `HELLO` (with the one time token) first, then any number of `RECORDS` and
//! `PROGRESS` frames, then `DONE` or `ERROR`. All integers are little endian.

pub const VERSION: u32 = 1;
/// Largest frame we accept, so a broken peer can not make us allocate gigabytes.
pub const MAX_FRAME: usize = 16 << 20;
/// Target payload size for one record batch.
pub const BATCH_BYTES: usize = 256 << 10;

pub const KIND_HELLO: u8 = 1;
pub const KIND_RECORDS: u8 = 2;
pub const KIND_PROGRESS: u8 = 3;
pub const KIND_DONE: u8 = 4;
pub const KIND_ERROR: u8 = 5;

/// Record flag: the entry is a folder.
pub const FLAG_DIR: u8 = 1;
/// MFT record number of the volume root folder.
pub const ROOT_RECORD: u64 = 5;

pub const ERR_NOT_NTFS: u16 = 1;
pub const ERR_OPEN_VOLUME: u16 = 2;
pub const ERR_READ: u16 = 3;
pub const ERR_ARGS: u16 = 4;
pub const ERR_INTERNAL: u16 = 5;

/// One name of one file. A file with several hard links sends one record per link, all with
/// the same `rec` and `size`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireRecord<'a> {
    /// MFT record number (the low 48 bits of the file reference).
    pub rec: u64,
    pub parent: u64,
    /// Allocated bytes on disk, summed over data streams.
    pub size: u64,
    /// Unix ms, `i64::MIN` when unknown.
    pub modified: i64,
    /// `FILE_ATTRIBUTE_*` from `$STANDARD_INFORMATION`.
    pub attributes: u32,
    pub flags: u8,
    pub name: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame {
    Hello { version: u32, token: String },
    Records { count: u32, payload: Vec<u8> },
    Progress { done: u64, total: u64 },
    Done { records: u64 },
    Error { code: u16, message: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireError {
    TooLarge,
    Malformed,
    UnknownKind(u8),
}

impl std::fmt::Display for WireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WireError::TooLarge => write!(f, "frame too large"),
            WireError::Malformed => write!(f, "malformed frame"),
            WireError::UnknownKind(k) => write!(f, "unknown frame kind {k}"),
        }
    }
}

impl std::error::Error for WireError {}

fn frame(out: &mut Vec<u8>, kind: u8, payload: &[u8]) {
    out.push(kind);
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(payload);
}

pub fn encode_hello(out: &mut Vec<u8>, token: &str) {
    let mut p = Vec::with_capacity(8 + token.len());
    p.extend_from_slice(&VERSION.to_le_bytes());
    p.extend_from_slice(&(token.len() as u16).to_le_bytes());
    p.extend_from_slice(token.as_bytes());
    frame(out, KIND_HELLO, &p);
}

pub fn encode_progress(out: &mut Vec<u8>, done: u64, total: u64) {
    let mut p = [0u8; 16];
    p[..8].copy_from_slice(&done.to_le_bytes());
    p[8..].copy_from_slice(&total.to_le_bytes());
    frame(out, KIND_PROGRESS, &p);
}

pub fn encode_done(out: &mut Vec<u8>, records: u64) {
    frame(out, KIND_DONE, &records.to_le_bytes());
}

pub fn encode_error(out: &mut Vec<u8>, code: u16, message: &str) {
    let msg = &message.as_bytes()[..message.len().min(1024)];
    let mut p = Vec::with_capacity(4 + msg.len());
    p.extend_from_slice(&code.to_le_bytes());
    p.extend_from_slice(&(msg.len() as u16).to_le_bytes());
    p.extend_from_slice(msg);
    frame(out, KIND_ERROR, &p);
}

/// Collects records until the batch is big enough to send.
#[derive(Debug, Default)]
pub struct RecordBatch {
    buf: Vec<u8>,
    count: u32,
}

impl RecordBatch {
    pub fn push(&mut self, r: &WireRecord) {
        let name = &r.name.as_bytes()[..r.name.len().min(u16::MAX as usize)];
        self.buf.extend_from_slice(&r.rec.to_le_bytes());
        self.buf.extend_from_slice(&r.parent.to_le_bytes());
        self.buf.extend_from_slice(&r.size.to_le_bytes());
        self.buf.extend_from_slice(&r.modified.to_le_bytes());
        self.buf.extend_from_slice(&r.attributes.to_le_bytes());
        self.buf.push(r.flags);
        self.buf.extend_from_slice(&(name.len() as u16).to_le_bytes());
        self.buf.extend_from_slice(name);
        self.count += 1;
    }

    pub fn count(&self) -> u32 {
        self.count
    }

    pub fn is_full(&self) -> bool {
        self.buf.len() >= BATCH_BYTES
    }

    /// Appends the batch as a frame to `out` and empties it.
    pub fn flush_into(&mut self, out: &mut Vec<u8>) {
        if self.count == 0 {
            return;
        }
        let mut p = Vec::with_capacity(4 + self.buf.len());
        p.extend_from_slice(&self.count.to_le_bytes());
        p.extend_from_slice(&self.buf);
        frame(out, KIND_RECORDS, &p);
        self.buf.clear();
        self.count = 0;
    }
}

struct Reader<'a> {
    b: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], WireError> {
        let s = self.b.get(self.pos..self.pos + n).ok_or(WireError::Malformed)?;
        self.pos += n;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, WireError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, WireError> {
        let s = self.take(2)?;
        Ok(u16::from_le_bytes([s[0], s[1]]))
    }
    fn u32(&mut self) -> Result<u32, WireError> {
        let s = self.take(4)?;
        Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }
    fn u64(&mut self) -> Result<u64, WireError> {
        let s = self.take(8)?;
        let mut a = [0u8; 8];
        a.copy_from_slice(s);
        Ok(u64::from_le_bytes(a))
    }
    fn str(&mut self, n: usize) -> Result<&'a str, WireError> {
        std::str::from_utf8(self.take(n)?).map_err(|_| WireError::Malformed)
    }
}

fn parse_frame(kind: u8, p: &[u8]) -> Result<Frame, WireError> {
    let mut r = Reader { b: p, pos: 0 };
    Ok(match kind {
        KIND_HELLO => {
            let version = r.u32()?;
            let n = r.u16()? as usize;
            Frame::Hello { version, token: r.str(n)?.to_string() }
        }
        KIND_RECORDS => {
            let count = r.u32()?;
            Frame::Records { count, payload: p[4..].to_vec() }
        }
        KIND_PROGRESS => Frame::Progress { done: r.u64()?, total: r.u64()? },
        KIND_DONE => Frame::Done { records: r.u64()? },
        KIND_ERROR => {
            let code = r.u16()?;
            let n = r.u16()? as usize;
            Frame::Error { code, message: String::from_utf8_lossy(r.take(n)?).into_owned() }
        }
        k => return Err(WireError::UnknownKind(k)),
    })
}

/// Turns a byte stream into frames. Feed it whatever the pipe returns.
#[derive(Debug, Default)]
pub struct FrameDecoder {
    buf: Vec<u8>,
    pos: usize,
}

impl FrameDecoder {
    pub fn push(&mut self, bytes: &[u8]) {
        if self.pos > 0 && self.pos == self.buf.len() {
            self.buf.clear();
            self.pos = 0;
        } else if self.pos > (1 << 20) {
            self.buf.drain(..self.pos);
            self.pos = 0;
        }
        self.buf.extend_from_slice(bytes);
    }

    /// The next complete frame, `Ok(None)` when more bytes are needed.
    pub fn next_frame(&mut self) -> Result<Option<Frame>, WireError> {
        let avail = &self.buf[self.pos..];
        if avail.len() < 5 {
            return Ok(None);
        }
        let kind = avail[0];
        let len = u32::from_le_bytes([avail[1], avail[2], avail[3], avail[4]]) as usize;
        if len > MAX_FRAME {
            return Err(WireError::TooLarge);
        }
        if avail.len() < 5 + len {
            return Ok(None);
        }
        let f = parse_frame(kind, &avail[5..5 + len])?;
        self.pos += 5 + len;
        Ok(Some(f))
    }
}

/// Iterates the records inside a `Frame::Records` payload.
pub struct RecordIter<'a> {
    r: Reader<'a>,
    left: u32,
}

pub fn records(count: u32, payload: &[u8]) -> RecordIter<'_> {
    RecordIter { r: Reader { b: payload, pos: 0 }, left: count }
}

impl<'a> Iterator for RecordIter<'a> {
    type Item = Result<WireRecord<'a>, WireError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.left == 0 {
            return None;
        }
        self.left -= 1;
        let mut one = || -> Result<WireRecord<'a>, WireError> {
            let rec = self.r.u64()?;
            let parent = self.r.u64()?;
            let size = self.r.u64()?;
            let modified = self.r.u64()? as i64;
            let attributes = self.r.u32()?;
            let flags = self.r.u8()?;
            let n = self.r.u16()? as usize;
            let name = self.r.str(n)?;
            Ok(WireRecord { rec, parent, size, modified, attributes, flags, name })
        };
        let item = one();
        if item.is_err() {
            self.left = 0;
        }
        Some(item)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_all_frames() {
        let mut out = Vec::new();
        encode_hello(&mut out, "abc123");
        let mut batch = RecordBatch::default();
        let r1 = WireRecord { rec: 5, parent: 5, size: 0, modified: 1, attributes: 0x10, flags: FLAG_DIR, name: "." };
        let r2 = WireRecord {
            rec: 1234,
            parent: 5,
            size: 1 << 40,
            modified: i64::MIN,
            attributes: 0x20,
            flags: 0,
            name: "فایل.txt",
        };
        batch.push(&r1);
        batch.push(&r2);
        batch.flush_into(&mut out);
        batch.flush_into(&mut out); // empty batch writes nothing
        encode_progress(&mut out, 10, 20);
        encode_error(&mut out, ERR_READ, "disk said no");
        encode_done(&mut out, 2);

        // Feed it in awkward chunks to test reassembly.
        let mut d = FrameDecoder::default();
        let mut frames = Vec::new();
        for chunk in out.chunks(3) {
            d.push(chunk);
            while let Some(f) = d.next_frame().expect("frame") {
                frames.push(f);
            }
        }
        assert_eq!(frames.len(), 5);
        assert_eq!(frames[0], Frame::Hello { version: VERSION, token: "abc123".into() });
        let Frame::Records { count, payload } = &frames[1] else { panic!("records") };
        let recs: Vec<_> = records(*count, payload).collect::<Result<_, _>>().expect("records");
        assert_eq!(recs, vec![r1, r2]);
        assert_eq!(frames[2], Frame::Progress { done: 10, total: 20 });
        assert_eq!(frames[3], Frame::Error { code: ERR_READ, message: "disk said no".into() });
        assert_eq!(frames[4], Frame::Done { records: 2 });
    }

    #[test]
    fn rejects_garbage() {
        let mut d = FrameDecoder::default();
        d.push(&[9, 0, 0, 0, 0]);
        assert_eq!(d.next_frame(), Err(WireError::UnknownKind(9)));

        let mut d = FrameDecoder::default();
        d.push(&[KIND_RECORDS, 0xFF, 0xFF, 0xFF, 0xFF]);
        assert_eq!(d.next_frame(), Err(WireError::TooLarge));

        // Record count says 2 but only one fits.
        let mut b = RecordBatch::default();
        b.push(&WireRecord { rec: 1, parent: 5, size: 1, modified: 0, attributes: 0, flags: 0, name: "a" });
        let mut out = Vec::new();
        b.flush_into(&mut out);
        let payload = out[9..].to_vec();
        let items: Vec<_> = records(2, &payload).collect();
        assert_eq!(items.len(), 2);
        assert!(items[1].is_err());
    }
}
