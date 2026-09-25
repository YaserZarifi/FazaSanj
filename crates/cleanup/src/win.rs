//! Every Win32 call of the cleanup engine lives here.

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, LocalFree, ERROR_CANCELLED, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW;
use windows_sys::Win32::Security::{GetTokenInformation, TokenUser, TOKEN_QUERY, TOKEN_USER};
use windows_sys::Win32::Storage::FileSystem::{
    FindClose, FindFirstFileW, GetDiskFreeSpaceExW, FILE_ATTRIBUTE_REPARSE_POINT, WIN32_FIND_DATAW,
};
use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, GetExitCodeProcess, OpenProcessToken, WaitForSingleObject, INFINITE,
};
use windows_sys::Win32::UI::Shell::{
    SHFileOperationW, ShellExecuteExW, ShellExecuteW, FOF_ALLOWUNDO, FOF_NOCONFIRMATION,
    FOF_NOERRORUI, FOF_SILENT, FOF_WANTNUKEWARNING, FO_DELETE, SEE_MASK_NOASYNC,
    SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, SHFILEOPSTRUCTW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{SW_HIDE, SW_SHOWNORMAL};

use crate::error::CleanupError;

fn wide(s: &OsStr) -> Vec<u16> {
    s.encode_wide().chain(std::iter::once(0)).collect()
}

/// The shell does not understand `\\?\` paths.
fn shell_path(p: &Path) -> Vec<u16> {
    let s = p.as_os_str().to_string_lossy();
    let plain = if let Some(rest) = s.strip_prefix("\\\\?\\UNC\\") {
        format!("\\\\{rest}")
    } else if let Some(rest) = s.strip_prefix("\\\\?\\") {
        rest.to_string()
    } else {
        s.into_owned()
    };
    OsStr::new(&plain).encode_wide().collect()
}

/// Sends the given paths to the Recycle Bin in one shell call. The caller checks afterwards
/// which paths are really gone, because the shell may stop halfway.
///
/// FOF_WANTNUKEWARNING makes the shell ask before it would delete something permanently
/// (item too big for the bin, or a drive without one) instead of doing it silently.
pub fn recycle(paths: &[&Path]) -> Result<(), CleanupError> {
    if paths.is_empty() {
        return Ok(());
    }
    let mut from: Vec<u16> = Vec::new();
    for p in paths {
        from.extend(shell_path(p));
        from.push(0);
    }
    from.push(0);
    let flags = FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_SILENT | FOF_NOERRORUI | FOF_WANTNUKEWARNING;
    let mut op = SHFILEOPSTRUCTW {
        hwnd: std::ptr::null_mut(),
        wFunc: FO_DELETE,
        pFrom: from.as_ptr(),
        pTo: std::ptr::null(),
        fFlags: flags as u16,
        fAnyOperationsAborted: 0,
        hNameMappings: std::ptr::null_mut(),
        lpszProgressTitle: std::ptr::null(),
    };
    // SAFETY: `op` is fully initialized, `from` is a double null terminated list that outlives
    // the call, and pTo is null as FO_DELETE requires.
    let rc = unsafe { SHFileOperationW(&mut op) };
    if rc != 0 {
        return Err(CleanupError::Win32(rc as u32));
    }
    if op.fAnyOperationsAborted != 0 {
        return Err(CleanupError::Cancelled);
    }
    Ok(())
}

/// Reparse tag of a reparse point, read without opening (and so without following) it.
pub fn reparse_tag(path: &Path) -> Option<u32> {
    let w = wide(path.as_os_str());
    // SAFETY: WIN32_FIND_DATAW is plain data, all zero is a valid value.
    let mut data: WIN32_FIND_DATAW = unsafe { std::mem::zeroed() };
    // SAFETY: `w` is null terminated and `data` is a valid out pointer.
    let h = unsafe { FindFirstFileW(w.as_ptr(), &mut data) };
    if h == INVALID_HANDLE_VALUE {
        return None;
    }
    // SAFETY: `h` is a valid find handle returned above.
    unsafe { FindClose(h) };
    if data.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        Some(data.dwReserved0)
    } else {
        None
    }
}

/// Free bytes available to the current user on the volume holding `root`.
pub fn free_space(root: &Path) -> Option<u64> {
    let w = wide(root.as_os_str());
    let mut avail = 0u64;
    let mut total = 0u64;
    let mut free = 0u64;
    // SAFETY: `w` is null terminated, the out pointers are valid u64s.
    let ok = unsafe { GetDiskFreeSpaceExW(w.as_ptr(), &mut avail, &mut total, &mut free) };
    (ok != 0).then_some(avail)
}

pub fn system_dir() -> Option<PathBuf> {
    let mut buf = [0u16; 512];
    // SAFETY: the buffer is valid for `buf.len()` u16s.
    let n = unsafe { GetSystemDirectoryW(buf.as_mut_ptr(), buf.len() as u32) } as usize;
    if n == 0 || n >= buf.len() {
        return None;
    }
    Some(PathBuf::from(String::from_utf16_lossy(&buf[..n])))
}

/// Starts `exe` with `params`, elevated when `elevated` is set, and waits for it to exit.
/// Returns the exit code. A refused UAC prompt becomes `UacRefused`.
pub fn run_and_wait(exe: &Path, params: &str, elevated: bool, show: bool) -> Result<u32, CleanupError> {
    let file = wide(exe.as_os_str());
    let params_w = wide(OsStr::new(params));
    let verb = wide(OsStr::new(if elevated { "runas" } else { "open" }));
    // SAFETY: SHELLEXECUTEINFOW is plain data, all zero is valid.
    let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    info.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC;
    info.lpVerb = verb.as_ptr();
    info.lpFile = file.as_ptr();
    info.lpParameters = params_w.as_ptr();
    info.nShow = if show { SW_SHOWNORMAL } else { SW_HIDE };
    // SAFETY: all string pointers are null terminated and outlive the call.
    let ok = unsafe { ShellExecuteExW(&mut info) };
    if ok == 0 {
        // SAFETY: no other call in between.
        let err = unsafe { GetLastError() };
        return Err(if err == ERROR_CANCELLED { CleanupError::UacRefused } else { CleanupError::Win32(err) });
    }
    let process: HANDLE = info.hProcess;
    if process.is_null() {
        return Err(CleanupError::Win32(0));
    }
    let mut code = 0u32;
    // SAFETY: `process` is a process handle we own (SEE_MASK_NOCLOSEPROCESS) and close once.
    unsafe {
        WaitForSingleObject(process, INFINITE);
        let got = GetExitCodeProcess(process, &mut code);
        let err = GetLastError();
        CloseHandle(process);
        if got == 0 {
            return Err(CleanupError::Win32(err));
        }
    }
    Ok(code)
}

/// Opens a URI or file with its default handler, without waiting.
pub fn shell_open(target: &str) -> Result<(), CleanupError> {
    let t = wide(OsStr::new(target));
    let verb = wide(OsStr::new("open"));
    // SAFETY: both strings are null terminated and outlive the call; other pointers may be null.
    let r = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            t.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    // ShellExecuteW returns a fake HINSTANCE, greater than 32 means success.
    let v = r as isize;
    if v > 32 {
        Ok(())
    } else {
        Err(CleanupError::Win32(v as u32))
    }
}

/// String SID of the user running this process, like `S-1-5-21-...`.
pub fn current_user_sid() -> Option<String> {
    let mut token: HANDLE = std::ptr::null_mut();
    // SAFETY: GetCurrentProcess returns a pseudo handle, `token` is a valid out pointer.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return None;
    }
    let mut len = 0u32;
    // SAFETY: first call only asks for the needed size.
    unsafe { GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut len) };
    // u64 buffer keeps TOKEN_USER properly aligned.
    let mut buf = vec![0u64; (len as usize).div_ceil(8).max(1)];
    // SAFETY: `buf` holds at least `len` bytes.
    let ok = unsafe { GetTokenInformation(token, TokenUser, buf.as_mut_ptr().cast(), len, &mut len) };
    // SAFETY: `token` came from OpenProcessToken.
    unsafe { CloseHandle(token) };
    if ok == 0 {
        return None;
    }
    // SAFETY: on success the buffer starts with a TOKEN_USER.
    let user = unsafe { &*(buf.as_ptr() as *const TOKEN_USER) };
    let mut s: *mut u16 = std::ptr::null_mut();
    // SAFETY: the SID pointer points into `buf`, which is still alive.
    if unsafe { ConvertSidToStringSidW(user.User.Sid, &mut s) } == 0 || s.is_null() {
        return None;
    }
    // SAFETY: `s` is a null terminated string allocated by the system, freed with LocalFree.
    let out = unsafe {
        let mut n = 0usize;
        while *s.add(n) != 0 {
            n += 1;
        }
        let v = String::from_utf16_lossy(std::slice::from_raw_parts(s, n));
        LocalFree(s.cast());
        v
    };
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sid_looks_right() {
        let sid = current_user_sid().unwrap();
        assert!(sid.starts_with("S-1-"), "{sid}");
    }

    #[test]
    fn system_dir_exists() {
        assert!(system_dir().unwrap().join("cmd.exe").exists());
    }

    #[test]
    fn free_space_of_temp() {
        let d = tempfile::tempdir().unwrap();
        assert!(free_space(d.path()).unwrap() > 0);
    }

    #[test]
    fn shell_path_strips_verbatim() {
        let v = shell_path(Path::new("\\\\?\\C:\\a"));
        assert_eq!(String::from_utf16_lossy(&v), "C:\\a");
    }
}
