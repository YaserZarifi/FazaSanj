//! Win32 helpers: volumes, file ids, allocated sizes, elevation.
//!
//! Every raw Windows call in the app lives in this crate.

mod drives;
mod wide;

pub use drives::list_drives;
pub use wide::{from_wide, to_wide};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("win32 error {code} in {context}")]
    Win32 { code: u32, context: &'static str },
    #[error("not supported on this platform")]
    Unsupported,
}

pub type Result<T> = std::result::Result<T, PlatformError>;

#[cfg(windows)]
pub(crate) fn last_error(context: &'static str) -> PlatformError {
    // SAFETY: GetLastError has no preconditions.
    let code = unsafe { windows_sys::Win32::Foundation::GetLastError() };
    PlatformError::Win32 { code, context }
}
