//! Owned Win32 handle. UNSAFE MODULE: wraps raw handles.

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};

/// Closes the handle on drop.
#[derive(Debug)]
pub struct OwnedHandle(HANDLE);

// SAFETY: kernel handles can be used and closed from any thread.
unsafe impl Send for OwnedHandle {}
// SAFETY: the calls we make through a shared handle are thread safe at the kernel level.
unsafe impl Sync for OwnedHandle {}

impl OwnedHandle {
    /// Takes ownership. Returns `None` for null and `INVALID_HANDLE_VALUE`.
    pub(crate) fn new(h: HANDLE) -> Option<Self> {
        if h.is_null() || h == INVALID_HANDLE_VALUE {
            None
        } else {
            Some(Self(h))
        }
    }

    pub(crate) fn raw(&self) -> HANDLE {
        self.0
    }

    /// Gives up ownership without closing.
    pub(crate) fn into_raw(self) -> HANDLE {
        let h = self.0;
        std::mem::forget(self);
        h
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // SAFETY: we own a valid handle and close it exactly once.
        unsafe {
            CloseHandle(self.0);
        }
    }
}
