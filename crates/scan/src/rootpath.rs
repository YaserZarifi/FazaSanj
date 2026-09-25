//! Normalizing the folder the user picked.

use std::path::Path;

use fazasanj_platform::{strip_long_prefix, to_long_path};

use crate::options::ScanError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanRoot {
    /// What the user sees: `C:\` or `D:\Projects` (no trailing backslash except drive roots).
    pub display: String,
    /// `\\?\` form for Win32 calls.
    pub verbatim: String,
    /// Drive letter when the root is on one, like `C`.
    pub drive: Option<char>,
}

/// Display form of a path: absolute, backslashes, no verbatim prefix, drive roots end in `\`.
pub fn display_path(path: &Path) -> String {
    let abs = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let mut s = strip_long_prefix(&abs.to_string_lossy()).replace('/', "\\");
    while s.len() > 3 && s.ends_with('\\') {
        s.pop();
    }
    if s.len() == 2 && s.as_bytes()[1] == b':' {
        s.push('\\');
    }
    s
}

pub fn resolve_root(path: &Path) -> Result<ScanRoot, ScanError> {
    let display = display_path(path);
    let meta = std::fs::metadata(to_long_path(Path::new(&display))).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => ScanError::RootNotFound(display.clone()),
        std::io::ErrorKind::PermissionDenied => ScanError::RootAccessDenied(display.clone()),
        _ => ScanError::Io(format!("{display}: {e}")),
    })?;
    if !meta.is_dir() {
        return Err(ScanError::NotADirectory(display));
    }
    let b = display.as_bytes();
    let drive = (b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':').then(|| (b[0] as char).to_ascii_uppercase());
    let verbatim = to_long_path(Path::new(&display)).to_string_lossy().into_owned();
    Ok(ScanRoot { display, verbatim, drive })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_forms() {
        assert_eq!(display_path(Path::new("C:")), r"C:\");
        assert_eq!(display_path(Path::new(r"C:\")), r"C:\");
        assert_eq!(display_path(Path::new(r"C:\Windows\")), r"C:\Windows");
        assert_eq!(display_path(Path::new(r"\\?\C:\Windows")), r"C:\Windows");
        assert_eq!(display_path(Path::new("C:/Users/x/")), r"C:\Users\x");
    }

    #[test]
    fn missing_root() {
        let r = resolve_root(Path::new(r"C:\no\such\folder\here"));
        assert!(matches!(r, Err(ScanError::RootNotFound(_))));
    }

    #[test]
    fn file_is_not_a_root() {
        let dir = tempfile::tempdir().expect("tempdir");
        let f = dir.path().join("x.txt");
        std::fs::write(&f, b"x").expect("write");
        assert!(matches!(resolve_root(&f), Err(ScanError::NotADirectory(_))));
        let r = resolve_root(dir.path()).expect("root");
        assert!(r.verbatim.starts_with(r"\\?\"));
        assert_eq!(r.drive.is_some(), r.display.as_bytes()[1] == b':');
    }
}
