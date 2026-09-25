//! Finding bundled helper binaries at runtime.

use std::path::{Path, PathBuf};

/// The fast scan helper's base name, without extension or target triple.
pub const FAST_SCAN_HELPER: &str = "fast-scan-helper";

const TRIPLE: &str = "x86_64-pc-windows-msvc";

/// Candidate file names for a sidecar in one folder. Tauri bundles strip the target triple,
/// dev builds and `src-tauri/binaries` keep it.
fn candidates(dir: &Path, base: &str) -> [PathBuf; 2] {
    [dir.join(format!("{base}.exe")), dir.join(format!("{base}-{TRIPLE}.exe"))]
}

/// Looks for `base` next to the running exe, then one folder up (cargo puts tests and
/// examples in `target/<profile>/deps` or `examples`, the helper in `target/<profile>`).
pub fn find_sidecar(base: &str) -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    find_sidecar_near(&exe, base)
}

/// Same as `find_sidecar` but relative to a given exe path.
pub fn find_sidecar_near(exe: &Path, base: &str) -> Option<PathBuf> {
    let dir = exe.parent()?;
    let mut dirs = vec![dir.to_path_buf()];
    if let Some(up) = dir.parent() {
        dirs.push(up.to_path_buf());
    }
    dirs.iter().flat_map(|d| candidates(d, base)).find(|p| p.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_triple_name_and_parent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let deps = dir.path().join("deps");
        std::fs::create_dir(&deps).expect("mkdir");
        let exe = deps.join("app.exe");
        assert_eq!(find_sidecar_near(&exe, "helper"), None);

        let side = dir.path().join(format!("helper-{TRIPLE}.exe"));
        std::fs::write(&side, b"").expect("write");
        assert_eq!(find_sidecar_near(&exe, "helper"), Some(side));

        let plain = deps.join("helper.exe");
        std::fs::write(&plain, b"").expect("write");
        assert_eq!(find_sidecar_near(&exe, "helper"), Some(plain));
    }
}
