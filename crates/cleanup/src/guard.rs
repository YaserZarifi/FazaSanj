//! The two checks every destructive step goes through: the hard block list and, in debug
//! builds, the dev sandbox.

use std::path::{Path, PathBuf};

use fazasanj_safety::{check_sandbox_for, SafetyContext};

use crate::error::CleanupError;

#[derive(Debug, Clone)]
pub(crate) struct Guard {
    safety: SafetyContext,
    sandbox: Option<PathBuf>,
    debug: bool,
}

impl Guard {
    pub fn new(safety: SafetyContext, sandbox: Option<PathBuf>, debug: bool) -> Self {
        Self { safety, sandbox, debug }
    }

    /// Block list and sandbox. Call right before touching `path`.
    pub fn check(&self, path: &Path) -> Result<(), CleanupError> {
        self.safety.check(path).map_err(CleanupError::Blocked)?;
        self.check_sandbox(path)
    }

    /// Sandbox only. Used for official system commands, whose target paths are on the block
    /// list on purpose (the command handles them, we never touch the files).
    pub fn check_sandbox(&self, path: &Path) -> Result<(), CleanupError> {
        check_sandbox_for(self.debug, path, self.sandbox.as_deref()).map_err(CleanupError::Sandbox)
    }

    pub fn check_block_list(&self, path: &Path) -> Result<(), CleanupError> {
        self.safety.check(path).map_err(CleanupError::Blocked)
    }

    pub fn debug(&self) -> bool {
        self.debug
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_without_sandbox_refuses() {
        let g = Guard::new(SafetyContext::new(vec![], vec![], None), None, true);
        let e = g.check(Path::new(r"D:\x\y")).unwrap_err();
        assert_eq!(e.code(), "sandbox_not_configured");
    }

    #[test]
    fn release_allows_outside_sandbox() {
        let g = Guard::new(SafetyContext::new(vec![], vec![], None), None, false);
        assert!(g.check(Path::new(r"D:\x\y")).is_ok());
        assert_eq!(g.check(Path::new("D:\\")).unwrap_err().code(), "blocked_drive_root");
    }
}
