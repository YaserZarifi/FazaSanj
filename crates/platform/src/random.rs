//! OS random numbers for pipe names and tokens. UNSAFE MODULE (Win32 call).

use windows_sys::Win32::Security::Cryptography::{BCryptGenRandom, BCRYPT_USE_SYSTEM_PREFERRED_RNG};

use crate::{PlatformError, Result};

/// Fills `buf` from the system CSPRNG.
pub fn random_bytes(buf: &mut [u8]) -> Result<()> {
    let len = u32::try_from(buf.len()).map_err(|_| PlatformError::InvalidInput("random length"))?;
    // SAFETY: `buf` is writable for `len` bytes; a null algorithm handle is allowed with this flag.
    let status = unsafe { BCryptGenRandom(std::ptr::null_mut(), buf.as_mut_ptr(), len, BCRYPT_USE_SYSTEM_PREFERRED_RNG) };
    if status < 0 {
        return Err(PlatformError::Win32 { code: status as u32, context: "BCryptGenRandom" });
    }
    Ok(())
}

/// `n` random bytes as lowercase hex (2n chars).
pub fn random_hex(n: usize) -> Result<String> {
    let mut buf = vec![0u8; n];
    random_bytes(&mut buf)?;
    Ok(buf.iter().map(|b| format!("{b:02x}")).collect())
}

#[cfg(test)]
mod tests {
    #[test]
    fn hex_is_random() {
        let a = super::random_hex(16).expect("random");
        let b = super::random_hex(16).expect("random");
        assert_eq!(a.len(), 32);
        assert_ne!(a, b);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
