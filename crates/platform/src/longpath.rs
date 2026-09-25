//! `\\?\` long path handling. Pure string work.

use std::path::{Path, PathBuf};

const VERBATIM: &str = r"\\?\";
const VERBATIM_UNC: &str = r"\\?\UNC\";

/// Turns an absolute path into its `\\?\` form so Win32 calls skip the 260 char limit.
///
/// Relative paths are resolved against the current directory first. Forward slashes become
/// backslashes because the verbatim form does no normalization at all.
pub fn to_long_path(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if s.starts_with(VERBATIM) || s.starts_with(r"\\.\") {
        return path.to_path_buf();
    }
    let abs = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let s = abs.to_string_lossy().replace('/', "\\");
    if let Some(rest) = s.strip_prefix(r"\\") {
        return PathBuf::from(format!("{VERBATIM_UNC}{rest}"));
    }
    PathBuf::from(format!("{VERBATIM}{s}"))
}

/// Removes a `\\?\` or `\\?\UNC\` prefix for display.
pub fn strip_long_prefix(path: &str) -> String {
    if let Some(rest) = path.strip_prefix(VERBATIM_UNC) {
        return format!(r"\\{rest}");
    }
    path.strip_prefix(VERBATIM).unwrap_or(path).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drive_paths() {
        assert_eq!(to_long_path(Path::new(r"C:\Windows")), PathBuf::from(r"\\?\C:\Windows"));
        assert_eq!(to_long_path(Path::new("C:/a/b")), PathBuf::from(r"\\?\C:\a\b"));
    }

    #[test]
    fn unc_and_already_long() {
        assert_eq!(to_long_path(Path::new(r"\\srv\share\x")), PathBuf::from(r"\\?\UNC\srv\share\x"));
        assert_eq!(to_long_path(Path::new(r"\\?\D:\x")), PathBuf::from(r"\\?\D:\x"));
    }

    #[test]
    fn strip() {
        assert_eq!(strip_long_prefix(r"\\?\C:\x"), r"C:\x");
        assert_eq!(strip_long_prefix(r"\\?\UNC\srv\s"), r"\\srv\s");
        assert_eq!(strip_long_prefix(r"C:\x"), r"C:\x");
    }
}
