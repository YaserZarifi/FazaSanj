//! Launching a helper through UAC and waiting for it. UNSAFE MODULE (Win32 calls).

use std::path::Path;
use std::time::Duration;

use thiserror::Error;
use windows_sys::Win32::Foundation::{
    GetLastError, ERROR_CANCELLED, ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows_sys::Win32::System::Threading::{GetExitCodeProcess, TerminateProcess, WaitForSingleObject, INFINITE};
use windows_sys::Win32::UI::Shell::{
    ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;

use crate::handle::OwnedHandle;
use crate::{last_error, to_wide, Result};

const STILL_ACTIVE: u32 = 259;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ElevateError {
    /// The user clicked "No" on the UAC prompt.
    #[error("the user declined the UAC prompt")]
    UacRefused,
    #[error("helper executable not found")]
    NotFound,
    #[error("win32 error {0} while launching the helper")]
    Win32(u32),
}

/// A process started through `launch_elevated`.
#[derive(Debug)]
pub struct ElevatedChild {
    process: OwnedHandle,
}

impl ElevatedChild {
    /// Waits up to `timeout` (forever for `None`). Returns the exit code, or `None` if still running.
    pub fn wait(&self, timeout: Option<Duration>) -> Result<Option<u32>> {
        let ms = timeout.map_or(INFINITE, |d| u32::try_from(d.as_millis()).unwrap_or(INFINITE - 1));
        // SAFETY: we own a valid process handle.
        let r = unsafe { WaitForSingleObject(self.process.raw(), ms) };
        match r {
            WAIT_OBJECT_0 => self.exit_code(),
            WAIT_TIMEOUT => Ok(None),
            _ => Err(last_error("WaitForSingleObject")),
        }
    }

    /// Exit code if the process has ended, without waiting.
    pub fn exit_code(&self) -> Result<Option<u32>> {
        let mut code = 0u32;
        // SAFETY: valid handle and out pointer.
        let ok = unsafe { GetExitCodeProcess(self.process.raw(), &mut code) };
        if ok == 0 {
            return Err(last_error("GetExitCodeProcess"));
        }
        if code == STILL_ACTIVE {
            // STILL_ACTIVE can also be a real exit code, so confirm with a zero wait.
            // SAFETY: valid handle.
            if unsafe { WaitForSingleObject(self.process.raw(), 0) } == WAIT_TIMEOUT {
                return Ok(None);
            }
        }
        Ok(Some(code))
    }

    /// Best effort kill. The handle UAC gives back may lack terminate rights, so this can fail.
    pub fn terminate(&self) -> bool {
        // SAFETY: valid handle.
        unsafe { TerminateProcess(self.process.raw(), 1) != 0 }
    }
}

/// Quotes arguments the way `CommandLineToArgvW` splits them.
pub fn quote_args<S: AsRef<str>>(args: &[S]) -> String {
    let mut out = String::new();
    for (i, a) in args.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        let a = a.as_ref();
        if !a.is_empty() && !a.contains([' ', '\t', '"']) {
            out.push_str(a);
            continue;
        }
        out.push('"');
        let mut backslashes = 0usize;
        for c in a.chars() {
            match c {
                '\\' => backslashes += 1,
                '"' => {
                    out.extend(std::iter::repeat_n('\\', backslashes * 2 + 1));
                    out.push('"');
                    backslashes = 0;
                }
                _ => {
                    out.extend(std::iter::repeat_n('\\', backslashes));
                    out.push(c);
                    backslashes = 0;
                }
            }
        }
        out.extend(std::iter::repeat_n('\\', backslashes * 2));
        out.push('"');
    }
    out
}

/// Starts `exe` with the "runas" verb, which shows the UAC prompt when needed.
///
/// Blocks until the user answers the prompt. Saying No gives `ElevateError::UacRefused`.
pub fn launch_elevated<S: AsRef<str>>(exe: &Path, args: &[S]) -> std::result::Result<ElevatedChild, ElevateError> {
    let verb = to_wide("runas");
    let file = to_wide(exe);
    let params = to_wide(quote_args(args));
    let dir = exe.parent().map(to_wide);

    // SAFETY: SHELLEXECUTEINFOW is plain data; all-zero is a valid starting state.
    let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    info.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI;
    info.lpVerb = verb.as_ptr();
    info.lpFile = file.as_ptr();
    info.lpParameters = params.as_ptr();
    info.lpDirectory = dir.as_ref().map_or(std::ptr::null(), |d| d.as_ptr());
    info.nShow = SW_HIDE;

    // SAFETY: every string pointer stays alive until the call returns.
    let ok = unsafe { ShellExecuteExW(&mut info) };
    if ok == 0 {
        // SAFETY: no preconditions.
        return Err(match unsafe { GetLastError() } {
            ERROR_CANCELLED => ElevateError::UacRefused,
            ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => ElevateError::NotFound,
            code => ElevateError::Win32(code),
        });
    }
    let process = OwnedHandle::new(info.hProcess).ok_or(ElevateError::Win32(0))?;
    Ok(ElevatedChild { process })
}

#[cfg(test)]
mod tests {
    use super::quote_args;

    #[test]
    fn quoting() {
        assert_eq!(quote_args(&["--volume", "C:"]), "--volume C:");
        assert_eq!(quote_args(&["a b"]), "\"a b\"");
        assert_eq!(quote_args(&[""]), "\"\"");
        assert_eq!(quote_args(&[r#"say "hi""#]), r#""say \"hi\"""#);
        assert_eq!(quote_args(&[r"C:\dir with space\"]), r#""C:\dir with space\\""#);
    }
}
