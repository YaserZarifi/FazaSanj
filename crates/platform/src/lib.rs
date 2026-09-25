//! Win32 helpers: volumes, file ids, allocated sizes, elevation, pipes.
//!
//! Every raw Windows call in the app lives in this crate. Modules marked UNSAFE MODULE hold
//! the `unsafe` blocks; the rest is plain Rust.

mod attrs;
mod drives;
mod longpath;
mod sidecar;
mod time;
mod wide;

#[cfg(windows)]
mod dirread;
#[cfg(windows)]
mod elevation;
#[cfg(windows)]
mod fileinfo;
#[cfg(windows)]
mod handle;
#[cfg(windows)]
mod pipe;
#[cfg(windows)]
mod random;
#[cfg(windows)]
mod rawvol;
#[cfg(windows)]
mod token;
#[cfg(windows)]
mod volume;

pub use attrs::*;
pub use drives::list_drives;
pub use longpath::{strip_long_prefix, to_long_path};
pub use sidecar::{find_sidecar, find_sidecar_near, FAST_SCAN_HELPER};
pub use time::filetime_to_unix_ms;
pub use wide::{from_wide, to_wide};

#[cfg(windows)]
pub use dirread::{read_dir, DirBuffer, DirEntry, DirError};
#[cfg(windows)]
pub use elevation::{launch_elevated, quote_args, ElevateError, ElevatedChild};
#[cfg(windows)]
pub use fileinfo::{compressed_size, file_identity, file_meta, standard_info, FileIdentity, StandardInfo};
#[cfg(windows)]
pub use pipe::{connect_pipe, is_valid_pipe_name, pipe_path, PipeError, PipeServer};
#[cfg(windows)]
pub use random::{random_bytes, random_hex};
#[cfg(windows)]
pub use rawvol::{file_extents, Extent, NtfsVolumeData, RawVolume};
#[cfg(windows)]
pub use token::{current_user_sid, is_elevated};
#[cfg(windows)]
pub use volume::{disk_space, filesystem_name, volume_info, volume_root, DiskSpace, VolumeInfo};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("win32 error {code} in {context}")]
    Win32 { code: u32, context: &'static str },
    #[error("invalid input: {0}")]
    InvalidInput(&'static str),
    #[error("not supported on this platform")]
    Unsupported,
}

impl PlatformError {
    /// The Win32 error code, if this came from a failed call.
    pub fn win32_code(&self) -> Option<u32> {
        match self {
            PlatformError::Win32 { code, .. } => Some(*code),
            _ => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, PlatformError>;

#[cfg(windows)]
pub(crate) fn last_error(context: &'static str) -> PlatformError {
    // SAFETY: GetLastError has no preconditions.
    let code = unsafe { windows_sys::Win32::Foundation::GetLastError() };
    PlatformError::Win32 { code, context }
}
