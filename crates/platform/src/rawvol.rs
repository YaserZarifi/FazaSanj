//! Read only raw volume access for the fast scan helper. UNSAFE MODULE (Win32 calls).
//!
//! Needs admin. Nothing here writes to the volume: the handle is opened with GENERIC_READ only.

use std::path::Path;

use windows_sys::Win32::Foundation::{GetLastError, ERROR_HANDLE_EOF, ERROR_MORE_DATA, GENERIC_READ};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, ReadFile, FILE_FLAG_BACKUP_SEMANTICS, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::Ioctl::{FSCTL_GET_NTFS_VOLUME_DATA, FSCTL_GET_RETRIEVAL_POINTERS};
use windows_sys::Win32::System::IO::{DeviceIoControl, OVERLAPPED};

use crate::handle::OwnedHandle;
use crate::longpath::to_long_path;
use crate::{last_error, to_wide, PlatformError, Result};

/// The parts of `NTFS_VOLUME_DATA_BUFFER` the MFT reader needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NtfsVolumeData {
    pub volume_serial: u64,
    pub total_clusters: u64,
    pub bytes_per_sector: u32,
    pub bytes_per_cluster: u32,
    pub bytes_per_record: u32,
    pub mft_valid_data_length: u64,
    pub mft_start_lcn: u64,
}

/// A run of clusters. `lcn` is `None` for a sparse hole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Extent {
    pub vcn: u64,
    pub lcn: Option<u64>,
    pub clusters: u64,
}

/// A volume opened for raw reading, like `\\.\C:`.
pub struct RawVolume {
    handle: OwnedHandle,
}

fn u32_at(b: &[u8], off: usize) -> u32 {
    b.get(off..off + 4).map_or(0, |s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

fn i64_at(b: &[u8], off: usize) -> i64 {
    b.get(off..off + 8).map_or(0, |s| {
        let mut a = [0u8; 8];
        a.copy_from_slice(s);
        i64::from_le_bytes(a)
    })
}

/// Parses `NTFS_VOLUME_DATA_BUFFER`.
pub(crate) fn parse_ntfs_volume_data(b: &[u8]) -> NtfsVolumeData {
    NtfsVolumeData {
        volume_serial: i64_at(b, 0) as u64,
        total_clusters: i64_at(b, 16) as u64,
        bytes_per_sector: u32_at(b, 40),
        bytes_per_cluster: u32_at(b, 44),
        bytes_per_record: u32_at(b, 48),
        mft_valid_data_length: i64_at(b, 56) as u64,
        mft_start_lcn: i64_at(b, 64) as u64,
    }
}

/// Parses `RETRIEVAL_POINTERS_BUFFER`. Returns the extents and the VCN after the last one.
pub(crate) fn parse_retrieval_pointers(b: &[u8], valid_len: usize) -> (Vec<Extent>, u64) {
    let b = &b[..valid_len.min(b.len())];
    let count = u32_at(b, 0) as usize;
    let mut vcn = i64_at(b, 8) as u64;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let off = 16 + i * 16;
        if off + 16 > b.len() {
            break;
        }
        let next = i64_at(b, off) as u64;
        let lcn = i64_at(b, off + 8);
        out.push(Extent { vcn, lcn: (lcn >= 0).then_some(lcn as u64), clusters: next.saturating_sub(vcn) });
        vcn = next;
    }
    (out, vcn)
}

fn open(path: &[u16], access: u32, flags: u32) -> Result<OwnedHandle> {
    // SAFETY: `path` is null terminated and outlives the call.
    let h = unsafe {
        CreateFileW(
            path.as_ptr(),
            access,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            flags,
            std::ptr::null_mut(),
        )
    };
    OwnedHandle::new(h).ok_or_else(|| last_error("CreateFileW"))
}

impl RawVolume {
    /// Opens `\\.\X:` read only. `letter` must be A to Z.
    pub fn open(letter: char) -> Result<Self> {
        if !letter.is_ascii_alphabetic() {
            return Err(PlatformError::InvalidInput("drive letter"));
        }
        let path = to_wide(format!(r"\\.\{}:", letter.to_ascii_uppercase()));
        Ok(Self { handle: open(&path, GENERIC_READ, 0)? })
    }

    /// Cluster size, record size and where the MFT starts.
    pub fn ntfs_data(&self) -> Result<NtfsVolumeData> {
        let mut buf = [0u64; 32];
        let mut n = 0u32;
        // SAFETY: output buffer is valid for its size; no input buffer.
        let ok = unsafe {
            DeviceIoControl(
                self.handle.raw(),
                FSCTL_GET_NTFS_VOLUME_DATA,
                std::ptr::null(),
                0,
                buf.as_mut_ptr().cast(),
                std::mem::size_of_val(&buf) as u32,
                &mut n,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(last_error("FSCTL_GET_NTFS_VOLUME_DATA"));
        }
        let bytes: Vec<u8> = buf.iter().flat_map(|w| w.to_le_bytes()).collect();
        Ok(parse_ntfs_volume_data(&bytes))
    }

    /// Reads at a byte offset. Offset and length must be sector aligned.
    pub fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize> {
        let len = u32::try_from(buf.len()).map_err(|_| PlatformError::InvalidInput("read length"))?;
        // SAFETY: OVERLAPPED is plain data; zero plus an offset is how positioned reads start.
        let mut ov: OVERLAPPED = unsafe { std::mem::zeroed() };
        ov.Anonymous.Anonymous.Offset = offset as u32;
        ov.Anonymous.Anonymous.OffsetHigh = (offset >> 32) as u32;
        let mut n = 0u32;
        // SAFETY: synchronous handle, so the read is complete when ReadFile returns; buffers are valid.
        let ok = unsafe { ReadFile(self.handle.raw(), buf.as_mut_ptr(), len, &mut n, &mut ov) };
        if ok == 0 {
            // SAFETY: no preconditions.
            if unsafe { GetLastError() } == ERROR_HANDLE_EOF {
                return Ok(0);
            }
            return Err(last_error("ReadFile(volume)"));
        }
        Ok(n as usize)
    }
}

/// Cluster runs of a file, via FSCTL_GET_RETRIEVAL_POINTERS. Used for `X:\$MFT`.
pub fn file_extents(path: &Path) -> Result<Vec<Extent>> {
    let w = to_wide(to_long_path(path));
    let h = open(&w, FILE_READ_ATTRIBUTES, FILE_FLAG_BACKUP_SEMANTICS)?;
    let mut out = Vec::new();
    let mut start: i64 = 0;
    let mut buf = vec![0u64; 8192];
    loop {
        let mut n = 0u32;
        // SAFETY: input is one i64 (STARTING_VCN_INPUT_BUFFER), output buffer is valid for its size.
        let ok = unsafe {
            DeviceIoControl(
                h.raw(),
                FSCTL_GET_RETRIEVAL_POINTERS,
                (&start as *const i64).cast(),
                8,
                buf.as_mut_ptr().cast(),
                (buf.len() * 8) as u32,
                &mut n,
                std::ptr::null_mut(),
            )
        };
        // SAFETY: no preconditions.
        let more = ok == 0 && unsafe { GetLastError() } == ERROR_MORE_DATA;
        if ok == 0 && !more {
            return Err(last_error("FSCTL_GET_RETRIEVAL_POINTERS"));
        }
        let bytes: Vec<u8> = buf.iter().flat_map(|w| w.to_le_bytes()).collect();
        let (extents, next) = parse_retrieval_pointers(&bytes, n as usize);
        let progressed = !extents.is_empty();
        out.extend(extents);
        if !more || !progressed {
            return Ok(out);
        }
        start = next as i64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retrieval_pointers() {
        let mut b = vec![0u8; 16 + 32];
        b[0..4].copy_from_slice(&2u32.to_le_bytes());
        b[8..16].copy_from_slice(&0i64.to_le_bytes());
        b[16..24].copy_from_slice(&100i64.to_le_bytes());
        b[24..32].copy_from_slice(&5000i64.to_le_bytes());
        b[32..40].copy_from_slice(&150i64.to_le_bytes());
        b[40..48].copy_from_slice(&(-1i64).to_le_bytes());
        let len = b.len();
        let (e, next) = parse_retrieval_pointers(&b, len);
        assert_eq!(next, 150);
        assert_eq!(e[0], Extent { vcn: 0, lcn: Some(5000), clusters: 100 });
        assert_eq!(e[1], Extent { vcn: 100, lcn: None, clusters: 50 });
    }

    #[test]
    fn volume_data_fields() {
        let mut b = vec![0u8; 96];
        b[44..48].copy_from_slice(&4096u32.to_le_bytes());
        b[48..52].copy_from_slice(&1024u32.to_le_bytes());
        b[64..72].copy_from_slice(&786_432i64.to_le_bytes());
        let d = parse_ntfs_volume_data(&b);
        assert_eq!((d.bytes_per_cluster, d.bytes_per_record, d.mft_start_lcn), (4096, 1024, 786_432));
    }

    /// Needs admin, so it does not run in normal test runs.
    #[test]
    #[ignore = "needs an elevated prompt"]
    fn read_system_volume_mft_extents() {
        let v = RawVolume::open('C').expect("open volume");
        let d = v.ntfs_data().expect("ntfs data");
        assert!(d.bytes_per_record >= 1024);
        let e = file_extents(Path::new(r"C:\$MFT")).expect("extents");
        assert!(!e.is_empty());
    }
}
