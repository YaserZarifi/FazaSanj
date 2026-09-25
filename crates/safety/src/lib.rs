//! The hard block list and the dev sandbox check.
//!
//! Every destructive code path must call [`SafetyContext::check`] (and [`check_sandbox`] in
//! debug builds) right before touching the disk. Nothing in the app can override this.

use std::path::Path;

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum BlockReason {
    #[error("path is empty, relative or contains '..'")]
    InvalidPath,
    #[error("path is the root of a drive or share")]
    DriveRoot,
    #[error("path is inside a protected system folder")]
    ProtectedTree,
    #[error("path contains a protected folder")]
    ContainsProtected,
    #[error("path is a user profile root")]
    ProfileRoot,
    #[error("path is a registry hive")]
    RegistryHive,
    #[error("item is managed by Windows and must be handled by its official tool")]
    SystemManaged,
}

impl BlockReason {
    pub fn code(self) -> &'static str {
        match self {
            BlockReason::InvalidPath => "blocked_invalid_path",
            BlockReason::DriveRoot => "blocked_drive_root",
            BlockReason::ProtectedTree => "blocked_protected",
            BlockReason::ContainsProtected => "blocked_contains_protected",
            BlockReason::ProfileRoot => "blocked_profile_root",
            BlockReason::RegistryHive => "blocked_registry_hive",
            BlockReason::SystemManaged => "blocked_system_managed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum SandboxError {
    #[error("no DEV_SANDBOX is set, destructive actions are disabled in debug builds")]
    NotConfigured,
    #[error("path is outside DEV_SANDBOX")]
    Outside,
}

impl SandboxError {
    pub fn code(self) -> &'static str {
        match self {
            SandboxError::NotConfigured => "sandbox_not_configured",
            SandboxError::Outside => "outside_sandbox",
        }
    }
}

/// Paths the block list protects, already normalized (lowercase, backslashes, no trailing slash).
#[derive(Debug, Clone)]
pub struct SafetyContext {
    /// Nothing inside these (or these themselves, or their parents) may be touched.
    protected_trees: Vec<String>,
    /// These exact folders (and their parents) may not be removed, but their children may.
    protected_exact: Vec<String>,
    /// Folder holding the user profiles, usually `c:\users`.
    profiles_dir: Option<String>,
}

const HIVE_PREFIXES: [&str; 2] = ["ntuser.dat", "usrclass.dat"];

/// Names at the root of a drive that are only ever handled through their official Windows tool.
const SYSTEM_MANAGED_ROOT_NAMES: [&str; 8] = [
    "hiberfil.sys",
    "pagefile.sys",
    "swapfile.sys",
    "windows.old",
    "$windows.~bt",
    "$windows.~ws",
    "system volume information",
    "$recycle.bin",
];

impl SafetyContext {
    /// Builds the context from the real environment of this machine.
    pub fn from_env(install_dir: Option<&Path>) -> Self {
        let env = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
        let system_drive = env("SystemDrive").unwrap_or_else(|| "C:".to_string());
        let windir = env("SystemRoot")
            .or_else(|| env("windir"))
            .unwrap_or_else(|| format!("{system_drive}\\Windows"));
        let profiles = env("PUBLIC")
            .and_then(|p| Path::new(&p).parent().map(|x| x.to_string_lossy().into_owned()))
            .unwrap_or_else(|| format!("{system_drive}\\Users"));

        let mut trees = vec![
            format!("{windir}\\System32"),
            format!("{windir}\\SysWOW64"),
            format!("{windir}\\WinSxS"),
            format!("{windir}\\servicing"),
            format!("{windir}\\Boot"),
            format!("{windir}\\Installer"),
            format!("{system_drive}\\Program Files"),
            format!("{system_drive}\\Program Files (x86)"),
            format!("{system_drive}\\Recovery"),
        ];
        for key in ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"] {
            if let Some(v) = env(key) {
                trees.push(v);
            }
        }
        if let Some(dir) = install_dir {
            trees.push(dir.to_string_lossy().into_owned());
        }
        let exact = vec![
            windir.clone(),
            format!("{system_drive}\\ProgramData"),
            profiles.clone(),
        ];
        Self::new(trees, exact, Some(profiles))
    }

    pub fn new(protected_trees: Vec<String>, protected_exact: Vec<String>, profiles_dir: Option<String>) -> Self {
        let norm = |v: Vec<String>| {
            let mut out: Vec<String> =
                v.iter().map(|p| normalize(Path::new(p))).filter(|p| !p.is_empty()).collect();
            out.sort();
            out.dedup();
            out
        };
        Self {
            protected_trees: norm(protected_trees),
            protected_exact: norm(protected_exact),
            profiles_dir: profiles_dir.map(|p| normalize(Path::new(&p))),
        }
    }

    /// Adds an extra protected tree (for example the real install folder, known at runtime).
    pub fn protect_tree(&mut self, path: &Path) {
        let p = normalize(path);
        if !p.is_empty() && !self.protected_trees.contains(&p) {
            self.protected_trees.push(p);
        }
    }

    /// Returns `Ok` only when deleting or changing `path` is allowed by the block list.
    pub fn check(&self, path: &Path) -> Result<(), BlockReason> {
        let raw = path.to_string_lossy();
        if raw.split(['\\', '/']).any(|c| c == "..") {
            return Err(BlockReason::InvalidPath);
        }
        let p = normalize(path);
        if !is_absolute(&p) {
            return Err(BlockReason::InvalidPath);
        }
        if is_drive_or_share_root(&p) {
            return Err(BlockReason::DriveRoot);
        }
        for t in &self.protected_trees {
            if is_same_or_inside(&p, t) {
                return Err(BlockReason::ProtectedTree);
            }
            if is_inside(t, &p) {
                return Err(BlockReason::ContainsProtected);
            }
        }
        for e in &self.protected_exact {
            if &p == e || is_inside(e, &p) {
                return Err(BlockReason::ContainsProtected);
            }
        }
        if let Some(profiles) = &self.profiles_dir {
            if parent_of(&p) == Some(profiles.as_str()) {
                return Err(BlockReason::ProfileRoot);
            }
        }
        let name = file_name(&p);
        if HIVE_PREFIXES.iter().any(|h| name.starts_with(h)) {
            return Err(BlockReason::RegistryHive);
        }
        if let Some(first) = root_component(&p) {
            if SYSTEM_MANAGED_ROOT_NAMES.contains(&first) {
                return Err(BlockReason::SystemManaged);
            }
        }
        Ok(())
    }

    pub fn is_blocked(&self, path: &Path) -> bool {
        self.check(path).is_err()
    }
}

/// In debug builds destructive actions only run inside the sandbox. Release builds allow all
/// (the block list still applies).
pub fn check_sandbox(path: &Path, sandbox: Option<&Path>) -> Result<(), SandboxError> {
    check_sandbox_for(cfg!(debug_assertions), path, sandbox)
}

pub fn check_sandbox_for(debug: bool, path: &Path, sandbox: Option<&Path>) -> Result<(), SandboxError> {
    if !debug {
        return Ok(());
    }
    let Some(sb) = sandbox else {
        return Err(SandboxError::NotConfigured);
    };
    let sb = normalize(sb);
    let p = normalize(path);
    let has_dotdot = path.to_string_lossy().split(['\\', '/']).any(|c| c == "..");
    if sb.is_empty() || has_dotdot || !is_same_or_inside(&p, sb.trim_end_matches('\\')) {
        return Err(SandboxError::Outside);
    }
    Ok(())
}

/// Lowercase, backslashes, no `\\?\` prefix, no repeated or trailing separators, no `.` parts.
pub fn normalize(path: &Path) -> String {
    let mut s = path.to_string_lossy().replace('/', "\\");
    if let Some(rest) = s.strip_prefix("\\\\?\\UNC\\") {
        s = format!("\\\\{rest}");
    } else if let Some(rest) = s.strip_prefix("\\\\?\\") {
        s = rest.to_string();
    }
    let s = s.to_lowercase();
    let unc = s.starts_with("\\\\");
    let parts: Vec<&str> = s.split('\\').filter(|c| !c.is_empty() && *c != ".").collect();
    let joined = parts.join("\\");
    if unc {
        format!("\\\\{joined}")
    } else {
        joined
    }
}

fn is_absolute(p: &str) -> bool {
    let b = p.as_bytes();
    (b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':') || p.starts_with("\\\\")
}

fn is_drive_or_share_root(p: &str) -> bool {
    let b = p.as_bytes();
    if b.len() == 2 && b[1] == b':' {
        return true;
    }
    if let Some(rest) = p.strip_prefix("\\\\") {
        // \\server or \\server\share
        return rest.split('\\').filter(|c| !c.is_empty()).count() <= 2;
    }
    false
}

fn is_inside(p: &str, dir: &str) -> bool {
    p.len() > dir.len() && p.starts_with(dir) && p.as_bytes()[dir.len()] == b'\\'
}

fn is_same_or_inside(p: &str, dir: &str) -> bool {
    p == dir || is_inside(p, dir)
}

fn parent_of(p: &str) -> Option<&str> {
    p.rfind('\\').map(|i| &p[..i])
}

fn file_name(p: &str) -> &str {
    p.rsplit('\\').next().unwrap_or(p)
}

/// First component after the drive (`c:\foo\bar` gives `foo`).
fn root_component(p: &str) -> Option<&str> {
    if p.starts_with("\\\\") {
        return p.trim_start_matches('\\').split('\\').nth(2);
    }
    p.split('\\').nth(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> SafetyContext {
        SafetyContext::new(
            vec![
                "C:\\Windows\\System32".into(),
                "C:\\Windows\\SysWOW64".into(),
                "C:\\Windows\\WinSxS".into(),
                "C:\\Program Files".into(),
                "C:\\Program Files (x86)".into(),
                "D:\\Apps\\Fazasanj".into(),
            ],
            vec!["C:\\Windows".into(), "C:\\ProgramData".into(), "C:\\Users".into()],
            Some("C:\\Users".into()),
        )
    }

    fn blocked(p: &str) -> bool {
        ctx().is_blocked(Path::new(p))
    }

    #[test]
    fn blocks_system_trees() {
        assert!(blocked("C:\\Windows\\System32"));
        assert!(blocked("c:\\windows\\system32\\drivers\\etc\\hosts"));
        assert!(blocked("C:\\Windows\\SysWOW64\\x.dll"));
        assert!(blocked("C:\\Program Files\\App"));
        assert!(blocked("C:\\Program Files (x86)\\App\\a.exe"));
        assert!(blocked("\\\\?\\C:\\Program Files\\App"));
        assert!(blocked("C:/Program Files/App"));
        assert!(blocked("C:\\Windows\\WinSxS\\Temp"));
        assert!(blocked("C:\\\\Windows\\\\System32\\"));
    }

    #[test]
    fn blocks_parents_of_protected() {
        assert!(blocked("C:\\Windows"));
        assert!(blocked("C:\\Users"));
        assert!(blocked("D:\\Apps"));
        assert!(blocked("C:\\ProgramData"));
    }

    #[test]
    fn blocks_roots_and_profiles() {
        assert!(blocked("C:\\"));
        assert!(blocked("C:"));
        assert!(blocked("T:\\"));
        assert!(blocked("\\\\server\\share"));
        assert!(blocked("\\\\server\\share\\"));
        assert!(blocked("C:\\Users\\yaser"));
        assert!(blocked("C:\\Users\\Public"));
    }

    #[test]
    fn blocks_hives_and_system_managed() {
        assert!(blocked("C:\\Users\\yaser\\NTUSER.DAT"));
        assert!(blocked("C:\\Users\\yaser\\ntuser.dat.LOG1"));
        assert!(blocked("C:\\Users\\yaser\\AppData\\Local\\Microsoft\\Windows\\UsrClass.dat"));
        assert!(blocked("C:\\hiberfil.sys"));
        assert!(blocked("C:\\pagefile.sys"));
        assert!(blocked("D:\\swapfile.sys"));
        assert!(blocked("C:\\Windows.old"));
        assert!(blocked("C:\\Windows.old\\Users"));
        assert!(blocked("C:\\$Windows.~BT"));
        assert!(blocked("C:\\System Volume Information"));
        assert!(blocked("E:\\$Recycle.Bin\\S-1-5-21"));
    }

    #[test]
    fn blocks_install_folder_and_bad_paths() {
        assert!(blocked("D:\\Apps\\Fazasanj\\fazasanj.exe"));
        assert!(blocked("relative\\path"));
        assert!(blocked(""));
        assert!(blocked("C:\\Users\\yaser\\AppData\\..\\..\\..\\Windows"));
    }

    #[test]
    fn allows_normal_junk() {
        assert!(!blocked("C:\\Users\\yaser\\AppData\\Local\\Temp"));
        assert!(!blocked("C:\\Users\\yaser\\AppData\\Roaming\\Telegram Desktop\\tdata\\user_data"));
        assert!(!blocked("C:\\Windows\\Temp"));
        assert!(!blocked("C:\\Windows\\SoftwareDistribution\\Download"));
        assert!(!blocked("T:\\junk\\node_modules"));
        assert!(!blocked("C:\\Program Files Extra\\x"));
        assert!(!blocked("C:\\ProgramData\\Package Cache\\x"));
        assert!(!blocked("C:\\hiberfil.sys.txt\\x"));
    }

    #[test]
    fn sandbox_rules() {
        let sb = Path::new("T:\\");
        assert!(check_sandbox_for(true, Path::new("T:\\junk"), Some(sb)).is_ok());
        assert_eq!(check_sandbox_for(true, Path::new("C:\\junk"), Some(sb)), Err(SandboxError::Outside));
        assert_eq!(check_sandbox_for(true, Path::new("T:\\junk"), None), Err(SandboxError::NotConfigured));
        assert_eq!(
            check_sandbox_for(true, Path::new("T:\\junk\\..\\..\\C:"), Some(sb)),
            Err(SandboxError::Outside)
        );
        assert!(check_sandbox_for(false, Path::new("C:\\junk"), None).is_ok());
        let sb2 = Path::new("T:\\sandbox");
        assert_eq!(check_sandbox_for(true, Path::new("T:\\sandbox2\\x"), Some(sb2)), Err(SandboxError::Outside));
    }

    #[test]
    fn from_env_blocks_real_system32() {
        let c = SafetyContext::from_env(None);
        let windir = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
        assert!(c.is_blocked(&Path::new(&windir).join("System32")));
    }
}
