//! Reads a folder tree before anything is removed, so dry runs and real runs make the same
//! decisions. Links are never followed, and every path goes through the guard.

use std::fs::OpenOptions;
use std::io::ErrorKind;
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::CleanupError;
use crate::guard::Guard;
use crate::win;

const ATTR_READONLY: u32 = 0x1;
const ATTR_DIRECTORY: u32 = 0x10;
const ATTR_REPARSE_POINT: u32 = 0x400;
const ATTR_OFFLINE: u32 = 0x1000;
const ATTR_RECALL_ON_OPEN: u32 = 0x4_0000;
const ATTR_RECALL_ON_DATA_ACCESS: u32 = 0x40_0000;

const TAG_NAME_SURROGATE: u32 = 0x2000_0000;
const TAG_CLOUD_MASK: u32 = 0xFFFF_0FFF;
const TAG_CLOUD: u32 = 0x9000_001A;

const DELETE_ACCESS: u32 = 0x0001_0000;
const SHARE_ALL: u32 = 0x1 | 0x2 | 0x4;
const FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
const FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;

const ERROR_ACCESS_DENIED: i32 = 5;
const ERROR_SHARING_VIOLATION: i32 = 32;
const ERROR_LOCK_VIOLATION: i32 = 33;

const MAX_DEPTH: u32 = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    File,
    Dir,
    /// Symlink or junction. Only the link itself is ever removed.
    Link { dir: bool },
}

#[derive(Debug)]
pub(crate) struct Entry {
    pub path: PathBuf,
    pub kind: Kind,
    pub size: u64,
    pub readonly: bool,
    /// Why this entry must be left alone. Nothing below it is touched either.
    pub problem: Option<CleanupError>,
    pub children: Vec<Entry>,
    /// Bytes and files in the subtree (links count as a file of size 0).
    pub total_bytes: u64,
    pub total_files: u64,
    /// No problems and no links anywhere below, so it can go to the Recycle Bin in one piece.
    pub whole: bool,
}

pub(crate) fn scan(path: &Path, guard: &Guard, cancel: &AtomicBool) -> Entry {
    scan_at(path.to_path_buf(), guard, cancel, 0)
}

fn scan_at(path: PathBuf, guard: &Guard, cancel: &AtomicBool, depth: u32) -> Entry {
    let mut e = Entry {
        path,
        kind: Kind::File,
        size: 0,
        readonly: false,
        problem: None,
        children: Vec::new(),
        total_bytes: 0,
        total_files: 0,
        whole: false,
    };
    if let Err(err) = guard.check(&e.path) {
        e.problem = Some(err);
        return e;
    }
    let meta = match std::fs::symlink_metadata(&e.path) {
        Ok(m) => m,
        Err(err) => {
            e.problem = Some(io_problem(&err));
            return e;
        }
    };
    let attrs = meta.file_attributes();
    let is_dir = attrs & ATTR_DIRECTORY != 0;
    e.readonly = attrs & ATTR_READONLY != 0;
    e.kind = if is_dir { Kind::Dir } else { Kind::File };

    if attrs & ATTR_REPARSE_POINT != 0 {
        match win::reparse_tag(&e.path) {
            Some(tag) if tag & TAG_NAME_SURROGATE != 0 => e.kind = Kind::Link { dir: is_dir },
            Some(tag) if tag & TAG_CLOUD_MASK == TAG_CLOUD => e.problem = Some(CleanupError::CloudFile),
            Some(_) => {}
            // Unknown reparse point we can not read: leave it.
            None => e.problem = Some(CleanupError::AccessDenied),
        }
    }
    if attrs & (ATTR_OFFLINE | ATTR_RECALL_ON_OPEN | ATTR_RECALL_ON_DATA_ACCESS) != 0 {
        // Deleting a cloud placeholder can delete the file in the cloud too.
        e.problem = Some(CleanupError::CloudFile);
    }
    if e.problem.is_some() {
        return e;
    }

    match e.kind {
        Kind::Link { .. } => {
            e.total_files = 1;
        }
        Kind::File => {
            e.size = meta.len();
            e.total_bytes = e.size;
            e.total_files = 1;
            e.problem = probe_delete(&e.path);
            e.whole = e.problem.is_none();
        }
        Kind::Dir => {
            if depth >= MAX_DEPTH {
                e.problem = Some(CleanupError::TooDeep);
                return e;
            }
            let rd = match std::fs::read_dir(&e.path) {
                Ok(rd) => rd,
                Err(err) => {
                    e.problem = Some(io_problem(&err));
                    return e;
                }
            };
            let mut paths: Vec<PathBuf> = Vec::new();
            for item in rd {
                match item {
                    Ok(d) => paths.push(d.path()),
                    Err(err) => {
                        e.problem = Some(io_problem(&err));
                        return e;
                    }
                }
            }
            paths.sort();
            let mut whole = true;
            for p in paths {
                if cancel.load(Ordering::Relaxed) {
                    e.problem = Some(CleanupError::Cancelled);
                    e.children.clear();
                    return e;
                }
                let c = scan_at(p, guard, cancel, depth + 1);
                e.total_bytes += c.total_bytes;
                e.total_files += c.total_files;
                whole &= c.whole;
                e.children.push(c);
            }
            e.whole = whole;
        }
    }
    e
}

/// Opens the file with delete access the same way a delete would, without deleting. Tells us
/// if another program holds it open without allowing deletes.
fn probe_delete(path: &Path) -> Option<CleanupError> {
    let r = OpenOptions::new()
        .access_mode(DELETE_ACCESS)
        .share_mode(SHARE_ALL)
        .custom_flags(FLAG_OPEN_REPARSE_POINT | FLAG_BACKUP_SEMANTICS)
        .open(path);
    r.err().map(|err| io_problem(&err))
}

pub(crate) fn io_problem(err: &std::io::Error) -> CleanupError {
    match err.raw_os_error() {
        Some(ERROR_SHARING_VIOLATION) | Some(ERROR_LOCK_VIOLATION) => CleanupError::InUse,
        Some(ERROR_ACCESS_DENIED) => CleanupError::AccessDenied,
        _ if err.kind() == ErrorKind::NotFound => CleanupError::NotFound,
        _ if err.kind() == ErrorKind::PermissionDenied => CleanupError::AccessDenied,
        _ => CleanupError::Io(err.to_string()),
    }
}
