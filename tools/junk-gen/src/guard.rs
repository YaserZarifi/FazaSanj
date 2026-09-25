//! Refuses targets where fake junk must never be written.

use std::path::{Path, PathBuf};

use crate::JunkError;

/// Marker file the generator writes at the target root. With `--force`, only folders that
/// have it may be written again.
pub const MARKER: &str = ".fazasanj-junk";

#[derive(Debug, Clone)]
pub struct Guard {
    /// Uppercase drive letters with colon, like `C:`.
    refused_drives: Vec<String>,
}

impl Guard {
    /// Refuses `C:` and the system drive of this machine.
    pub fn from_env() -> Self {
        let mut drives = vec!["C:".to_string()];
        if let Ok(sd) = std::env::var("SystemDrive") {
            let sd = sd.trim().to_ascii_uppercase();
            if !sd.is_empty() && !drives.contains(&sd) {
                drives.push(sd);
            }
        }
        Self { refused_drives: drives }
    }

    /// Used by tests, which can only use temp folders on the system drive.
    #[doc(hidden)]
    pub fn refusing(drives: &[&str]) -> Self {
        Self { refused_drives: drives.iter().map(|d| d.to_ascii_uppercase()).collect() }
    }

    pub fn check(&self, target: &Path, force: bool) -> Result<(), JunkError> {
        let raw = target.to_string_lossy();
        if raw.split(['\\', '/']).any(|c| c == "..") {
            return Err(JunkError::BadTarget(raw.into_owned()));
        }
        let drive = drive_of(&raw).ok_or_else(|| JunkError::BadTarget(raw.to_string()))?;
        if self.refused_drives.contains(&drive) {
            return Err(JunkError::SystemDrive(drive));
        }
        // A subst drive can point back to the system drive, so check the real path too.
        if let Ok(real) = std::fs::canonicalize(target) {
            if let Some(d) = drive_of(&real.to_string_lossy()) {
                if self.refused_drives.contains(&d) {
                    return Err(JunkError::SystemDrive(d));
                }
            }
        }
        if has_real_windows(target) {
            return Err(JunkError::RealWindows(target.display().to_string()));
        }
        if !target.exists() {
            return Ok(());
        }
        if !target.is_dir() {
            return Err(JunkError::BadTarget(raw.into_owned()));
        }
        let empty = std::fs::read_dir(target)
            .map_err(|e| JunkError::Io(target.display().to_string(), e))?
            .next()
            .is_none();
        if empty {
            return Ok(());
        }
        let ours = target.join(MARKER).is_file();
        match (ours, force) {
            (true, true) => Ok(()),
            (true, false) => Err(JunkError::NeedsForce(target.display().to_string())),
            (false, _) => Err(JunkError::NotEmpty(target.display().to_string())),
        }
    }
}

fn has_real_windows(target: &Path) -> bool {
    let sys = target.join("Windows").join("System32");
    sys.join("ntoskrnl.exe").exists() || sys.join("config").join("SYSTEM").exists()
}

/// `C:` from `C:\x`, `\\?\C:\x` or `c:/x`.
fn drive_of(p: &str) -> Option<String> {
    let p = p.strip_prefix(r"\\?\").unwrap_or(p);
    let b = p.as_bytes();
    let ok = b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b.len() == 2 || b[2] == b'\\' || b[2] == b'/');
    ok.then(|| p[..2].to_ascii_uppercase())
}

/// Absolute target with a trailing separator removed.
pub fn clean_target(p: &str) -> PathBuf {
    let t = p.trim_end_matches(['\\', '/']);
    if t.len() == 2 && t.ends_with(':') {
        PathBuf::from(format!("{t}\\"))
    } else {
        PathBuf::from(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_c_and_system_drive() {
        let g = Guard::from_env();
        assert!(matches!(g.check(Path::new(r"C:\"), false), Err(JunkError::SystemDrive(_))));
        assert!(matches!(g.check(Path::new(r"c:\junk"), false), Err(JunkError::SystemDrive(_))));
        let sd = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
        assert!(g.check(&PathBuf::from(format!(r"{sd}\junk")), false).is_err());
    }

    #[test]
    fn refuses_relative_and_unc() {
        let g = Guard::refusing(&["Q:"]);
        assert!(g.check(Path::new(r"junk\here"), false).is_err());
        assert!(g.check(Path::new(r"\\server\share\x"), false).is_err());
        assert!(g.check(Path::new(r"T:\a\..\b"), false).is_err());
    }

    #[test]
    fn drive_parsing() {
        assert_eq!(drive_of(r"\\?\t:\x").as_deref(), Some("T:"));
        assert_eq!(drive_of("T:").as_deref(), Some("T:"));
        assert_eq!(drive_of("T:x"), None);
        assert_eq!(clean_target("T:"), PathBuf::from(r"T:\"));
        assert_eq!(clean_target(r"T:\junk\"), PathBuf::from(r"T:\junk"));
    }
}
