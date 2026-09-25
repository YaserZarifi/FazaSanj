//! Current process token: user SID and elevation. UNSAFE MODULE (Win32 calls).

use windows_sys::Win32::Foundation::{LocalFree, HANDLE};
use windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW;
use windows_sys::Win32::Security::{GetTokenInformation, TokenElevation, TokenUser, TOKEN_QUERY};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

use crate::handle::OwnedHandle;
use crate::{last_error, PlatformError, Result};

fn process_token() -> Result<OwnedHandle> {
    let mut h: HANDLE = std::ptr::null_mut();
    // SAFETY: the pseudo handle from GetCurrentProcess needs no closing, `h` is a valid out pointer.
    let ok = unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut h) };
    if ok == 0 {
        return Err(last_error("OpenProcessToken"));
    }
    OwnedHandle::new(h).ok_or(PlatformError::InvalidInput("token handle"))
}

/// Reads a token information class into an 8 byte aligned buffer.
fn token_info(token: &OwnedHandle, class: i32) -> Result<Vec<u64>> {
    let mut needed = 0u32;
    // SAFETY: size query with a null buffer is allowed; it fails with ERROR_INSUFFICIENT_BUFFER.
    unsafe { GetTokenInformation(token.raw(), class, std::ptr::null_mut(), 0, &mut needed) };
    if needed == 0 {
        return Err(last_error("GetTokenInformation(size)"));
    }
    let mut buf = vec![0u64; (needed as usize).div_ceil(8)];
    // SAFETY: buf holds at least `needed` writable bytes.
    let ok = unsafe { GetTokenInformation(token.raw(), class, buf.as_mut_ptr().cast(), needed, &mut needed) };
    if ok == 0 {
        return Err(last_error("GetTokenInformation"));
    }
    Ok(buf)
}

/// SID of the user running this process, like `S-1-5-21-...`.
pub fn current_user_sid() -> Result<String> {
    let token = process_token()?;
    let buf = token_info(&token, TokenUser)?;
    // TOKEN_USER starts with SID_AND_ATTRIBUTES whose first field is the PSID.
    let sid = buf[0] as usize as *mut core::ffi::c_void;
    let mut out: *mut u16 = std::ptr::null_mut();
    // SAFETY: `sid` points into `buf`, which is alive for this call. `out` receives a LocalAlloc string.
    let ok = unsafe { ConvertSidToStringSidW(sid, &mut out) };
    if ok == 0 || out.is_null() {
        return Err(last_error("ConvertSidToStringSidW"));
    }
    // SAFETY: `out` is a valid null terminated UTF-16 string until LocalFree below.
    let s = unsafe {
        let mut len = 0usize;
        while *out.add(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(out, len))
    };
    // SAFETY: allocated by ConvertSidToStringSidW with LocalAlloc.
    unsafe { LocalFree(out.cast()) };
    Ok(s)
}

/// True when this process runs with an elevated (admin) token.
pub fn is_elevated() -> bool {
    let Ok(token) = process_token() else { return false };
    match token_info(&token, TokenElevation) {
        Ok(buf) => (buf[0] & 0xFFFF_FFFF) != 0,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn sid_looks_right() {
        let sid = super::current_user_sid().expect("sid");
        assert!(sid.starts_with("S-1-"), "{sid}");
    }

    #[test]
    fn elevation_query_does_not_fail() {
        let _ = super::is_elevated();
    }
}
