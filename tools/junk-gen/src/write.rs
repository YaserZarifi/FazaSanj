//! Writes a plan to disk. Only ever creates files, never deletes anything.

use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::plan::{Content, Entry, Kind, Plan};
use crate::rng::Rng;
use crate::JunkError;

const CHUNK: usize = 1 << 20;
const DAY: u64 = 86_400;

pub fn write_plan(root: &Path, plan: &Plan) -> Result<(), JunkError> {
    let io = |p: &Path, e: std::io::Error| JunkError::Io(p.display().to_string(), e);
    for e in &plan.entries {
        let path = root.join(&e.rel);
        match &e.kind {
            Kind::Dir => fs::create_dir_all(&path).map_err(|x| io(&path, x))?,
            Kind::File { size, content } => {
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).map_err(|x| io(parent, x))?;
                }
                write_file(&path, *size, content, plan).map_err(|x| io(&path, x))?;
            }
        }
    }
    // Times last, since creating children changes the folder times.
    for e in plan.entries.iter().rev() {
        if let Some(days) = e.age_days {
            set_age(&root.join(&e.rel), e, days);
        }
    }
    Ok(())
}

fn seed_of(content: &Content, plan: &Plan) -> (u64, bool) {
    let mut near = false;
    let mut c = content;
    // Follow copy chains to the original bytes.
    for _ in 0..8 {
        match c {
            Content::Random(s) => return (*s, near),
            Content::CopyOf(i) | Content::NearCopyOf(i) => {
                near |= matches!(c, Content::NearCopyOf(_));
                match plan.entries.get(*i).map(|e| &e.kind) {
                    Some(Kind::File { content, .. }) => c = content,
                    _ => return (0, near),
                }
            }
        }
    }
    (0, near)
}

fn write_file(path: &Path, size: u64, content: &Content, plan: &Plan) -> std::io::Result<()> {
    let (seed, near) = seed_of(content, plan);
    let mut rng = Rng::new(seed);
    let mut out = BufWriter::with_capacity(CHUNK, File::create(path)?);
    let mut buf = vec![0u8; CHUNK];
    let mut left = size;
    while left > 0 {
        let n = left.min(CHUNK as u64) as usize;
        rng.fill(&mut buf[..n]);
        if near && left == n as u64 {
            if let Some(last) = buf[..n].last_mut() {
                *last ^= 0x5a;
            }
        }
        out.write_all(&buf[..n])?;
        left -= n as u64;
    }
    out.flush()
}

fn set_age(path: &Path, e: &Entry, days: u32) {
    let when = SystemTime::now() - Duration::from_secs(u64::from(days) * DAY);
    let file = match e.kind {
        Kind::File { .. } => OpenOptions::new().write(true).open(path),
        Kind::Dir => open_dir_for_times(path),
    };
    // Best effort: a wrong folder time only makes the fake tree a bit less realistic.
    if let Ok(f) = file {
        let _ = f.set_modified(when);
    }
}

#[cfg(windows)]
fn open_dir_for_times(path: &Path) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_WRITE_ATTRIBUTES: u32 = 0x0100;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    OpenOptions::new().access_mode(FILE_WRITE_ATTRIBUTES).custom_flags(FILE_FLAG_BACKUP_SEMANTICS).open(path)
}

#[cfg(not(windows))]
fn open_dir_for_times(path: &Path) -> std::io::Result<File> {
    File::open(path)
}
