use std::ffi::OsStr;

/// UTF-16, null terminated, for Win32 `W` functions.
pub fn to_wide(s: impl AsRef<OsStr>) -> Vec<u16> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        s.as_ref().encode_wide().chain(std::iter::once(0)).collect()
    }
    #[cfg(not(windows))]
    {
        s.as_ref().to_string_lossy().encode_utf16().chain(std::iter::once(0)).collect()
    }
}

/// Reads a UTF-16 buffer up to the first null.
pub fn from_wide(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let w = to_wide("C:\\فضا");
        assert_eq!(*w.last().unwrap_or(&1), 0);
        assert_eq!(from_wide(&w), "C:\\فضا");
    }
}
