//! Named pipe between the app and the elevated helper. UNSAFE MODULE (Win32 calls).
//!
//! The server is inbound only (the helper writes, the app reads), accepts a single local
//! client, and its ACL allows only the current user (optionally also Administrators, which
//! is needed when UAC elevation happens with a different admin account).

use std::fs::File;
use std::os::windows::io::FromRawHandle;
use std::time::{Duration, Instant};

use thiserror::Error;
use windows_sys::Win32::Foundation::{
    GetLastError, LocalFree, ERROR_BROKEN_PIPE, ERROR_FILE_NOT_FOUND, ERROR_IO_PENDING, ERROR_NO_DATA,
    ERROR_PIPE_BUSY, ERROR_PIPE_CONNECTED, GENERIC_WRITE, WAIT_OBJECT_0,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, ReadFile, FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_FLAG_OVERLAPPED, OPEN_EXISTING, PIPE_ACCESS_INBOUND,
    SECURITY_IDENTIFICATION, SECURITY_SQOS_PRESENT,
};
use windows_sys::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, WaitNamedPipeW, PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS,
    PIPE_TYPE_BYTE, PIPE_WAIT,
};
use windows_sys::Win32::System::Threading::{CreateEventW, WaitForSingleObject};

use crate::handle::OwnedHandle;
use crate::token::current_user_sid;
use crate::{last_error, to_wide, PlatformError};

#[derive(Debug, Error)]
pub enum PipeError {
    #[error("timed out waiting on the pipe")]
    Timeout,
    #[error("aborted")]
    Aborted,
    #[error("invalid pipe name")]
    InvalidName,
    #[error(transparent)]
    Platform(#[from] PlatformError),
}

type PipeResult<T> = std::result::Result<T, PipeError>;

/// Only short names made of letters, digits, `-`, `_` and `.` are accepted.
pub fn is_valid_pipe_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 200
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
}

/// `\\.\pipe\<name>`
pub fn pipe_path(name: &str) -> String {
    format!(r"\\.\pipe\{name}")
}

fn sddl_for_current_user(allow_admins: bool) -> PipeResult<String> {
    let sid = current_user_sid()?;
    // Protected DACL, generic all for the user. Nothing for anyone else.
    let mut sddl = format!("D:P(A;;GA;;;{sid})");
    if allow_admins {
        sddl.push_str("(A;;GA;;;BA)");
    }
    Ok(sddl)
}

/// Server end of the pipe, created before the helper starts.
pub struct PipeServer {
    pipe: OwnedHandle,
    event: OwnedHandle,
    name: String,
}

impl PipeServer {
    /// Creates `\\.\pipe\<name>`. Fails if the name already exists, so nobody can grab it first.
    pub fn create(name: &str, allow_admins: bool) -> PipeResult<Self> {
        if !is_valid_pipe_name(name) {
            return Err(PipeError::InvalidName);
        }
        let sddl = to_wide(sddl_for_current_user(allow_admins)?);
        let mut sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        // SAFETY: `sddl` is null terminated, `sd` receives a LocalAlloc buffer that we free below.
        let ok = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut sd,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(last_error("ConvertStringSecurityDescriptorToSecurityDescriptorW").into());
        }
        let sa = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: sd,
            bInheritHandle: 0,
        };
        let path = to_wide(pipe_path(name));
        // SAFETY: `path` and `sa` are valid for the call; the descriptor is copied by the kernel.
        let h = unsafe {
            CreateNamedPipeW(
                path.as_ptr(),
                PIPE_ACCESS_INBOUND | FILE_FLAG_OVERLAPPED | FILE_FLAG_FIRST_PIPE_INSTANCE,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                1,
                0,
                1 << 20,
                0,
                &sa,
            )
        };
        let created = OwnedHandle::new(h).ok_or_else(|| last_error("CreateNamedPipeW"));
        // SAFETY: `sd` came from ConvertStringSecurityDescriptorToSecurityDescriptorW.
        unsafe { LocalFree(sd) };
        let pipe = created?;

        // SAFETY: manual reset, initially unsignaled, unnamed event.
        let ev = unsafe { CreateEventW(std::ptr::null(), 1, 0, std::ptr::null()) };
        let event = OwnedHandle::new(ev).ok_or_else(|| last_error("CreateEventW"))?;
        Ok(Self { pipe, event, name: name.to_string() })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    fn new_overlapped(&self) -> OVERLAPPED {
        // SAFETY: OVERLAPPED is plain data, zero is its documented initial state.
        let mut ov: OVERLAPPED = unsafe { std::mem::zeroed() };
        ov.hEvent = self.event.raw();
        ov
    }

    /// Waits for the pending operation on `ov`. On timeout or abort the I/O is cancelled and
    /// fully drained before returning, so `ov` never outlives a live request.
    fn finish(&self, ov: &mut OVERLAPPED, timeout: Duration, abort: &dyn Fn() -> bool) -> PipeResult<u32> {
        let deadline = Instant::now() + timeout;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let slice = left.min(Duration::from_millis(100)).as_millis() as u32;
            // SAFETY: valid event handle.
            let r = unsafe { WaitForSingleObject(self.event.raw(), slice) };
            if r == WAIT_OBJECT_0 {
                let mut n = 0u32;
                // SAFETY: the operation on `ov` has completed.
                let ok = unsafe { GetOverlappedResult(self.pipe.raw(), ov, &mut n, 0) };
                if ok == 0 {
                    // SAFETY: no preconditions.
                    let code = unsafe { GetLastError() };
                    if code == ERROR_BROKEN_PIPE || code == ERROR_NO_DATA {
                        return Ok(0);
                    }
                    return Err(PlatformError::Win32 { code, context: "GetOverlappedResult" }.into());
                }
                return Ok(n);
            }
            let aborted = abort();
            if aborted || Instant::now() >= deadline {
                let mut n = 0u32;
                // SAFETY: cancel then block until the kernel is done with `ov`.
                unsafe {
                    CancelIoEx(self.pipe.raw(), ov);
                    GetOverlappedResult(self.pipe.raw(), ov, &mut n, 1);
                }
                return Err(if aborted { PipeError::Aborted } else { PipeError::Timeout });
            }
        }
    }

    /// Waits until the helper connects.
    pub fn wait_for_client(&self, timeout: Duration, abort: &dyn Fn() -> bool) -> PipeResult<()> {
        let mut ov = self.new_overlapped();
        // SAFETY: `ov` lives until `finish` returns, which waits for completion.
        let ok = unsafe { ConnectNamedPipe(self.pipe.raw(), &mut ov) };
        if ok != 0 {
            return Ok(());
        }
        // SAFETY: no preconditions.
        match unsafe { GetLastError() } {
            ERROR_PIPE_CONNECTED => Ok(()),
            ERROR_IO_PENDING => self.finish(&mut ov, timeout, abort).map(|_| ()),
            code => Err(PlatformError::Win32 { code, context: "ConnectNamedPipe" }.into()),
        }
    }

    /// Reads what is available, up to `buf.len()`. `Ok(0)` means the client closed its end.
    pub fn read(&self, buf: &mut [u8], timeout: Duration, abort: &dyn Fn() -> bool) -> PipeResult<usize> {
        let len = u32::try_from(buf.len()).unwrap_or(u32::MAX);
        let mut ov = self.new_overlapped();
        // SAFETY: `buf` and `ov` stay alive until `finish` has drained the request.
        let ok = unsafe { ReadFile(self.pipe.raw(), buf.as_mut_ptr(), len, std::ptr::null_mut(), &mut ov) };
        if ok == 0 {
            // SAFETY: no preconditions.
            let code = unsafe { GetLastError() };
            if code == ERROR_BROKEN_PIPE || code == ERROR_NO_DATA {
                return Ok(0);
            }
            if code != ERROR_IO_PENDING {
                return Err(PlatformError::Win32 { code, context: "ReadFile(pipe)" }.into());
            }
        }
        // Completed right away or pending, either way the event gets signaled.
        self.finish(&mut ov, timeout, abort).map(|n| n as usize)
    }
}

/// Client end, used by the helper. Opens the pipe for writing with identification level
/// impersonation only, so the server can not act as the (elevated) client.
pub fn connect_pipe(name: &str, timeout: Duration) -> PipeResult<File> {
    if !is_valid_pipe_name(name) {
        return Err(PipeError::InvalidName);
    }
    let path = to_wide(pipe_path(name));
    let deadline = Instant::now() + timeout;
    loop {
        // SAFETY: `path` is null terminated.
        let h = unsafe {
            CreateFileW(
                path.as_ptr(),
                GENERIC_WRITE,
                0,
                std::ptr::null(),
                OPEN_EXISTING,
                SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION,
                std::ptr::null_mut(),
            )
        };
        if let Some(h) = OwnedHandle::new(h) {
            // SAFETY: we own the handle and hand it to File, which closes it.
            return Ok(unsafe { File::from_raw_handle(h.into_raw()) });
        }
        // SAFETY: no preconditions.
        let code = unsafe { GetLastError() };
        if Instant::now() >= deadline {
            return Err(PipeError::Timeout);
        }
        match code {
            // SAFETY: `path` is null terminated.
            ERROR_PIPE_BUSY => unsafe {
                WaitNamedPipeW(path.as_ptr(), 250);
            },
            ERROR_FILE_NOT_FOUND => std::thread::sleep(Duration::from_millis(50)),
            _ => return Err(PlatformError::Win32 { code, context: "CreateFileW(pipe)" }.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn names() {
        assert!(is_valid_pipe_name("fazasanj-0a1b"));
        assert!(!is_valid_pipe_name(r"..\evil"));
        assert!(!is_valid_pipe_name(""));
    }

    #[test]
    fn roundtrip_and_single_instance() {
        let name = format!("fazasanj-test-{}", crate::random_hex(8).expect("rand"));
        let server = PipeServer::create(&name, false).expect("create");
        assert!(PipeServer::create(&name, false).is_err(), "second instance must fail");

        let n2 = name.clone();
        let writer = std::thread::spawn(move || {
            let mut f = connect_pipe(&n2, Duration::from_secs(5)).expect("connect");
            f.write_all(b"hello pipe").expect("write");
        });
        server.wait_for_client(Duration::from_secs(5), &|| false).expect("client");
        let mut got = Vec::new();
        let mut buf = [0u8; 4];
        loop {
            let n = server.read(&mut buf, Duration::from_secs(5), &|| false).expect("read");
            if n == 0 {
                break;
            }
            got.extend_from_slice(&buf[..n]);
        }
        writer.join().expect("join");
        assert_eq!(got, b"hello pipe");
    }

    #[test]
    fn connect_times_out() {
        let name = format!("fazasanj-test-{}", crate::random_hex(8).expect("rand"));
        let server = PipeServer::create(&name, false).expect("create");
        let r = server.wait_for_client(Duration::from_millis(150), &|| false);
        assert!(matches!(r, Err(PipeError::Timeout)));
        let r = server.wait_for_client(Duration::from_secs(5), &|| true);
        assert!(matches!(r, Err(PipeError::Aborted)));
    }
}
