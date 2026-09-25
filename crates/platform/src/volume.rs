//! Volume level queries: root, filesystem, serial, free space. UNSAFE MODULE (Win32 calls).

use std::path::{Path, PathBuf};

use windows_sys::Win32::Storage::FileSystem::{GetDiskFreeSpaceExW, GetVolumeInformationW, GetVolumePathNameW};

use crate::longpath::strip_long_prefix;
use crate::{from_wide, last_error, to_wide, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VolumeInfo {
    /// Mount root such as `C:\`.
    pub root: PathBuf,
    pub label: String,
    /// "NTFS", "ReFS", "FAT32", "exFAT"...
    pub filesystem: String,
    pub serial: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiskSpace {
    pub total: u64,
    pub free: u64,
    /// Free space this user may use (quotas).
    pub free_to_caller: u64,
}

/// The mount point that contains `path`, for example `C:\` for `C:\Users\x`.
pub fn volume_root(path: &Path) -> Result<PathBuf> {
    let w = to_wide(path);
    let mut out = [0u16; 1024];
    // SAFETY: `w` is null terminated, `out` is valid for its length.
    let ok = unsafe { GetVolumePathNameW(w.as_ptr(), out.as_mut_ptr(), out.len() as u32) };
    if ok == 0 {
        return Err(last_error("GetVolumePathNameW"));
    }
    Ok(PathBuf::from(strip_long_prefix(&from_wide(&out))))
}

/// Label, filesystem name and serial of the volume holding `path`.
pub fn volume_info(path: &Path) -> Result<VolumeInfo> {
    let root = volume_root(path)?;
    let root_w = to_wide(&root);
    let mut label = [0u16; 261];
    let mut fs = [0u16; 261];
    let mut serial = 0u32;
    // SAFETY: all buffers are valid for their lengths, unused outputs are null.
    let ok = unsafe {
        GetVolumeInformationW(
            root_w.as_ptr(),
            label.as_mut_ptr(),
            label.len() as u32,
            &mut serial,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            fs.as_mut_ptr(),
            fs.len() as u32,
        )
    };
    if ok == 0 {
        return Err(last_error("GetVolumeInformationW"));
    }
    Ok(VolumeInfo { root, label: from_wide(&label), filesystem: from_wide(&fs), serial })
}

/// Filesystem name for the volume holding `path` ("NTFS", "ReFS"...).
pub fn filesystem_name(path: &Path) -> Result<String> {
    Ok(volume_info(path)?.filesystem)
}

/// Total and free bytes of the volume holding `path`.
pub fn disk_space(path: &Path) -> Result<DiskSpace> {
    let w = to_wide(path);
    let (mut caller, mut total, mut free) = (0u64, 0u64, 0u64);
    // SAFETY: `w` is null terminated, out pointers are valid.
    let ok = unsafe { GetDiskFreeSpaceExW(w.as_ptr(), &mut caller, &mut total, &mut free) };
    if ok == 0 {
        return Err(last_error("GetDiskFreeSpaceExW"));
    }
    Ok(DiskSpace { total, free, free_to_caller: caller })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temp_dir_volume() {
        let tmp = std::env::temp_dir();
        let info = volume_info(&tmp).expect("volume info");
        assert!(info.root.to_string_lossy().ends_with('\\'));
        assert!(!info.filesystem.is_empty());
        let space = disk_space(&tmp).expect("space");
        assert!(space.total > 0 && space.free <= space.total);
    }
}
