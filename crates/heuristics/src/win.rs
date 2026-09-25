//! The one Win32 call the heuristics need.

use std::fs::File;
use std::os::windows::io::AsRawHandle;

use windows_sys::Win32::Storage::FileSystem::{GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION};

/// Volume serial and file index. Two paths with the same id are hardlinks to one file.
pub(crate) fn file_id(file: &File) -> Option<(u32, u64)> {
    // SAFETY: BY_HANDLE_FILE_INFORMATION is plain data, all zero is valid.
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: the handle belongs to `file`, which is alive for the call, and `info` is a valid
    // out pointer.
    let ok = unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) };
    if ok == 0 {
        return None;
    }
    let index = (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow);
    Some((info.dwVolumeSerialNumber, index))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hardlinks_share_an_id() {
        let d = tempfile::tempdir().unwrap();
        let a = d.path().join("a");
        let b = d.path().join("b");
        let c = d.path().join("c");
        std::fs::write(&a, b"x").unwrap();
        std::fs::write(&c, b"x").unwrap();
        std::fs::hard_link(&a, &b).unwrap();
        let id = |p: &std::path::Path| file_id(&File::open(p).unwrap()).unwrap();
        assert_eq!(id(&a), id(&b));
        assert_ne!(id(&a), id(&c));
    }
}
