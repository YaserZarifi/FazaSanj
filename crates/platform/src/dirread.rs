//! Batched directory enumeration. UNSAFE MODULE (Win32 calls in `read_dir`).
//!
//! We use `GetFileInformationByHandleEx` with `FileIdExtdDirectoryInfo` instead of
//! `FindFirstFileExW`. Both fetch many entries per call, but the find API only returns the
//! logical size. This class also returns the allocation size and the 128 bit file id, which
//! is what hard link dedupe needs, without opening every file.

use std::path::Path;

use thiserror::Error;
use windows_sys::Win32::Foundation::{
    GetLastError, ERROR_ACCESS_DENIED, ERROR_FILE_NOT_FOUND, ERROR_INVALID_FUNCTION, ERROR_INVALID_LEVEL,
    ERROR_INVALID_PARAMETER, ERROR_NOT_SUPPORTED, ERROR_NO_MORE_FILES, ERROR_PATH_NOT_FOUND,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FileFullDirectoryInfo, FileFullDirectoryRestartInfo, FileIdExtdDirectoryInfo, FileIdExtdDirectoryRestartInfo,
    GetFileInformationByHandleEx, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_LIST_DIRECTORY,
    FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};

use crate::handle::OwnedHandle;
use crate::longpath::to_long_path;
use crate::to_wide;

/// One directory entry, borrowed from the enumeration buffer.
#[derive(Debug, Clone, Copy)]
pub struct DirEntry<'a> {
    pub name: &'a str,
    pub attributes: u32,
    /// Only meaningful when the reparse point attribute is set.
    pub reparse_tag: u32,
    /// Zero when the filesystem does not report ids (older FAT drivers).
    pub file_id: u128,
    pub allocation_size: u64,
    pub end_of_file: u64,
    /// FILETIME.
    pub last_write: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum DirError {
    #[error("access denied")]
    AccessDenied,
    #[error("not found")]
    NotFound,
    #[error("win32 error {0}")]
    Other(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DirClass {
    IdExtd,
    Full,
}

/// Reusable 64 KB aligned buffer. One per thread is enough.
pub struct DirBuffer {
    buf: Vec<u64>,
    name: String,
}

impl Default for DirBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl DirBuffer {
    pub fn new() -> Self {
        // 64 KB is the largest size that also works over SMB.
        Self { buf: vec![0u64; 64 * 1024 / 8], name: String::with_capacity(256) }
    }
}

fn u32_at(b: &[u8], off: usize) -> Option<u32> {
    b.get(off..off + 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

fn u64_at(b: &[u8], off: usize) -> Option<u64> {
    let s = b.get(off..off + 8)?;
    let mut a = [0u8; 8];
    a.copy_from_slice(s);
    Some(u64::from_le_bytes(a))
}

/// Walks the entries in one filled buffer. Returns false if `f` asked to stop.
/// Malformed input ends the walk quietly instead of reading out of bounds.
pub(crate) fn parse_entries(
    buf: &[u8],
    class: DirClass,
    name: &mut String,
    f: &mut impl FnMut(&DirEntry) -> bool,
) -> bool {
    // Field offsets of FILE_ID_EXTD_DIR_INFO and FILE_FULL_DIR_INFO.
    let name_off = match class {
        DirClass::IdExtd => 88,
        DirClass::Full => 68,
    };
    let mut pos = 0usize;
    loop {
        let Some(e) = buf.get(pos..) else { return true };
        let (Some(next), Some(name_len)) = (u32_at(e, 0), u32_at(e, 60)) else { return true };
        let name_len = name_len as usize;
        let Some(raw_name) = e.get(name_off..name_off + name_len) else { return true };

        name.clear();
        let units = raw_name.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c));
        name.extend(char::decode_utf16(units).map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER)));

        if name != "." && name != ".." {
            let (reparse_tag, file_id) = match class {
                DirClass::IdExtd => {
                    let lo = u64_at(e, 72).unwrap_or(0);
                    let hi = u64_at(e, 80).unwrap_or(0);
                    (u32_at(e, 68).unwrap_or(0), u128::from(lo) | (u128::from(hi) << 64))
                }
                // For reparse points the EaSize field carries the tag.
                DirClass::Full => (u32_at(e, 64).unwrap_or(0), 0),
            };
            let entry = DirEntry {
                name: name.as_str(),
                attributes: u32_at(e, 56).unwrap_or(0),
                reparse_tag,
                file_id,
                allocation_size: u64_at(e, 48).unwrap_or(0),
                end_of_file: u64_at(e, 40).unwrap_or(0),
                last_write: u64_at(e, 24).unwrap_or(0) as i64,
            };
            if !f(&entry) {
                return false;
            }
        }
        if next == 0 {
            return true;
        }
        pos += next as usize;
    }
}

fn open_dir(path: &Path, follow_reparse: bool) -> Result<OwnedHandle, DirError> {
    let w = to_wide(to_long_path(path));
    let mut flags = FILE_FLAG_BACKUP_SEMANTICS;
    if !follow_reparse {
        flags |= FILE_FLAG_OPEN_REPARSE_POINT;
    }
    // SAFETY: `w` is null terminated and outlives the call.
    let h = unsafe {
        CreateFileW(
            w.as_ptr(),
            FILE_LIST_DIRECTORY,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            flags,
            std::ptr::null_mut(),
        )
    };
    OwnedHandle::new(h).ok_or_else(|| {
        // SAFETY: no preconditions.
        match unsafe { GetLastError() } {
            ERROR_ACCESS_DENIED => DirError::AccessDenied,
            ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => DirError::NotFound,
            code => DirError::Other(code),
        }
    })
}

/// Lists one directory, calling `f` for every entry except `.` and `..`.
///
/// `f` returns false to stop early (for cancellation). With `follow_reparse` false a
/// reparse point directory is opened as itself, which also avoids cloud recalls.
pub fn read_dir(
    path: &Path,
    follow_reparse: bool,
    buf: &mut DirBuffer,
    mut f: impl FnMut(&DirEntry) -> bool,
) -> Result<(), DirError> {
    let h = open_dir(path, follow_reparse)?;
    let mut class = DirClass::IdExtd;
    let mut info_class = FileIdExtdDirectoryRestartInfo;
    let mut first = true;
    let len = (buf.buf.len() * 8) as u32;
    loop {
        // SAFETY: buf.buf is a writable, 8 byte aligned buffer of `len` bytes.
        let ok = unsafe { GetFileInformationByHandleEx(h.raw(), info_class, buf.buf.as_mut_ptr().cast(), len) };
        if ok == 0 {
            // SAFETY: no preconditions.
            let code = unsafe { GetLastError() };
            if code == ERROR_NO_MORE_FILES {
                return Ok(());
            }
            let unsupported = matches!(
                code,
                ERROR_INVALID_PARAMETER | ERROR_INVALID_LEVEL | ERROR_NOT_SUPPORTED | ERROR_INVALID_FUNCTION
            );
            if first && class == DirClass::IdExtd && unsupported {
                class = DirClass::Full;
                info_class = FileFullDirectoryRestartInfo;
                continue;
            }
            return Err(if code == ERROR_ACCESS_DENIED { DirError::AccessDenied } else { DirError::Other(code) });
        }
        if first && class == DirClass::IdExtd {
            info_class = FileIdExtdDirectoryInfo;
        } else if first {
            info_class = FileFullDirectoryInfo;
        }
        first = false;
        // SAFETY: viewing initialized u64s as bytes is always valid, the length matches.
        let bytes = unsafe { std::slice::from_raw_parts(buf.buf.as_ptr().cast::<u8>(), len as usize) };
        if !parse_entries(bytes, class, &mut buf.name, &mut f) {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn push_entry(out: &mut Vec<u8>, name: &str, alloc: u64, attrs: u32, id: u128, last: bool) {
        let start = out.len();
        let units: Vec<u16> = name.encode_utf16().collect();
        let mut e = vec![0u8; 88 + units.len() * 2];
        let total = e.len().div_ceil(8) * 8;
        e.resize(total, 0);
        let next = if last { 0 } else { total as u32 };
        e[0..4].copy_from_slice(&next.to_le_bytes());
        e[24..32].copy_from_slice(&5i64.to_le_bytes());
        e[40..48].copy_from_slice(&(alloc / 2).to_le_bytes());
        e[48..56].copy_from_slice(&alloc.to_le_bytes());
        e[56..60].copy_from_slice(&attrs.to_le_bytes());
        e[60..64].copy_from_slice(&((units.len() * 2) as u32).to_le_bytes());
        e[72..88].copy_from_slice(&id.to_le_bytes());
        for (i, u) in units.iter().enumerate() {
            e[88 + i * 2..90 + i * 2].copy_from_slice(&u.to_le_bytes());
        }
        out.extend_from_slice(&e);
        assert_eq!(start % 8, 0);
    }

    #[test]
    fn parses_extd_entries() {
        let mut buf = Vec::new();
        push_entry(&mut buf, ".", 0, 0x10, 1, false);
        push_entry(&mut buf, "فایل.txt", 4096, 0x20, 42, false);
        push_entry(&mut buf, "sub", 0, 0x10, 43, true);
        let mut seen = Vec::new();
        let mut name = String::new();
        let done = parse_entries(&buf, DirClass::IdExtd, &mut name, &mut |e: &DirEntry| {
            seen.push((e.name.to_string(), e.allocation_size, e.file_id, e.attributes, e.last_write));
            true
        });
        assert!(done);
        assert_eq!(seen.len(), 2);
        assert_eq!(seen[0], ("فایل.txt".to_string(), 4096, 42, 0x20, 5));
        assert_eq!(seen[1].0, "sub");
    }

    #[test]
    fn truncated_buffer_is_safe() {
        let mut buf = Vec::new();
        push_entry(&mut buf, "abc", 1, 0, 1, false);
        buf.truncate(buf.len() - 3);
        let mut name = String::new();
        let mut n = 0;
        parse_entries(&buf, DirClass::IdExtd, &mut name, &mut |_: &DirEntry| {
            n += 1;
            true
        });
        assert_eq!(n, 0);
    }

    #[test]
    fn lists_real_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("a.txt"), b"hello").expect("write");
        std::fs::create_dir(dir.path().join("sub")).expect("mkdir");
        let mut names = Vec::new();
        let mut b = DirBuffer::new();
        read_dir(dir.path(), false, &mut b, |e| {
            names.push((e.name.to_string(), e.end_of_file, e.file_id != 0));
            true
        })
        .expect("read_dir");
        names.sort();
        assert_eq!(names, vec![("a.txt".to_string(), 5, true), ("sub".to_string(), 0, true)]);
    }

    #[test]
    fn missing_directory() {
        let mut b = DirBuffer::new();
        let r = read_dir(Path::new(r"C:\definitely\not\here\x"), false, &mut b, |_| true);
        assert_eq!(r, Err(DirError::NotFound));
    }
}
