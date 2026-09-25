//! Per file metadata: identity, allocated size, link count. UNSAFE MODULE (Win32 calls).
//!
//! These open a handle per call, so they are much slower than directory enumeration.
//! Scanners should only use them for the few files where the directory entry is not enough.

use std::path::Path;

use windows_sys::Win32::Foundation::SetLastError;
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FileIdInfo, FileStandardInfo, GetCompressedFileSizeW, GetFileInformationByHandleEx,
    FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE,
    FILE_SHARE_READ, FILE_SHARE_WRITE, INVALID_FILE_SIZE, OPEN_EXISTING,
};

use crate::handle::OwnedHandle;
use crate::longpath::to_long_path;
use crate::{last_error, to_wide, PlatformError, Result};

/// Identifies a file across all its hard links.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileIdentity {
    pub volume_serial: u64,
    pub file_id: u128,
}

/// `FILE_STANDARD_INFO` without the Win32 types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StandardInfo {
    pub allocation_size: u64,
    pub end_of_file: u64,
    pub number_of_links: u32,
    pub delete_pending: bool,
    pub directory: bool,
}

/// Opens a file or folder for attribute queries only. Reparse points are not followed.
fn open_meta(path: &Path) -> Result<OwnedHandle> {
    let w = to_wide(to_long_path(path));
    // SAFETY: `w` is null terminated and outlives the call. No security attributes or template.
    let h = unsafe {
        CreateFileW(
            w.as_ptr(),
            FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            std::ptr::null_mut(),
        )
    };
    OwnedHandle::new(h).ok_or_else(|| last_error("CreateFileW"))
}

fn query(h: &OwnedHandle, class: i32, buf: &mut [u64], context: &'static str) -> Result<()> {
    let len = u32::try_from(std::mem::size_of_val(buf)).map_err(|_| PlatformError::InvalidInput("buffer"))?;
    // SAFETY: buf is a valid, 8 byte aligned writable buffer of `len` bytes.
    let ok = unsafe { GetFileInformationByHandleEx(h.raw(), class, buf.as_mut_ptr().cast(), len) };
    if ok == 0 {
        return Err(last_error(context));
    }
    Ok(())
}

fn identity_from(h: &OwnedHandle) -> Result<FileIdentity> {
    // FILE_ID_INFO is 24 bytes: u64 serial + 16 byte id.
    let mut buf = [0u64; 3];
    query(h, FileIdInfo, &mut buf, "GetFileInformationByHandleEx(FileIdInfo)")?;
    let lo = u128::from(buf[1]);
    let hi = u128::from(buf[2]);
    Ok(FileIdentity { volume_serial: buf[0], file_id: lo | (hi << 64) })
}

fn standard_from(h: &OwnedHandle) -> Result<StandardInfo> {
    // FILE_STANDARD_INFO: i64, i64, u32, BOOLEAN, BOOLEAN (+ padding) = 24 bytes.
    let mut buf = [0u64; 3];
    query(h, FileStandardInfo, &mut buf, "GetFileInformationByHandleEx(FileStandardInfo)")?;
    let tail = buf[2].to_le_bytes();
    Ok(StandardInfo {
        allocation_size: buf[0],
        end_of_file: buf[1],
        number_of_links: u32::from_le_bytes([tail[0], tail[1], tail[2], tail[3]]),
        delete_pending: tail[4] != 0,
        directory: tail[5] != 0,
    })
}

/// Volume serial plus the 128 bit file id. Every hard link of a file returns the same value.
pub fn file_identity(path: &Path) -> Result<FileIdentity> {
    identity_from(&open_meta(path)?)
}

/// Allocation size, logical size and hard link count.
pub fn standard_info(path: &Path) -> Result<StandardInfo> {
    standard_from(&open_meta(path)?)
}

/// Both of the above with a single open.
pub fn file_meta(path: &Path) -> Result<(FileIdentity, StandardInfo)> {
    let h = open_meta(path)?;
    Ok((identity_from(&h)?, standard_from(&h)?))
}

/// Bytes actually used on disk for compressed, sparse or WOF compressed files.
/// For normal files this equals the logical size, not the cluster rounded size.
pub fn compressed_size(path: &Path) -> Result<u64> {
    let w = to_wide(to_long_path(path));
    let mut high = 0u32;
    // SAFETY: `w` is null terminated, `high` is a valid out pointer. The last error is cleared
    // first because a low word of 0xFFFFFFFF is only an error when the call sets it.
    let low = unsafe {
        SetLastError(0);
        GetCompressedFileSizeW(w.as_ptr(), &mut high)
    };
    if low == INVALID_FILE_SIZE {
        let err = last_error("GetCompressedFileSizeW");
        if let PlatformError::Win32 { code, .. } = err {
            if code != 0 {
                return Err(err);
            }
        }
    }
    Ok((u64::from(high) << 32) | u64::from(low))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hardlinks_share_identity() {
        let dir = tempfile::tempdir().expect("tempdir");
        let a = dir.path().join("a.bin");
        let b = dir.path().join("b.bin");
        std::fs::write(&a, vec![7u8; 10_000]).expect("write");
        std::fs::hard_link(&a, &b).expect("hard link");

        let (ia, sa) = file_meta(&a).expect("meta a");
        let ib = file_identity(&b).expect("id b");
        assert_eq!(ia, ib);
        assert_eq!(sa.number_of_links, 2);
        assert_eq!(sa.end_of_file, 10_000);
        assert!(sa.allocation_size >= 10_000);
        assert!(!sa.directory);
        assert_eq!(compressed_size(&a).expect("compressed size"), 10_000);
    }

    #[test]
    fn directory_flag() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(standard_info(dir.path()).expect("std").directory);
    }
}
