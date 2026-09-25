use fazasanj_model::ApiError;
use fazasanj_safety::{BlockReason, SandboxError};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CleanupError {
    #[error("blocked by the safety list: {0}")]
    Blocked(BlockReason),
    #[error("blocked by the dev sandbox: {0}")]
    Sandbox(SandboxError),
    #[error("action was marked as blocked in the plan")]
    PlanBlocked,
    #[error("command is not on the allow list")]
    CommandNotAllowed,
    #[error("open target is not on the allow list")]
    TargetNotAllowed,
    #[error("the admin prompt was refused")]
    UacRefused,
    #[error("the virtual disk is in use")]
    VhdxInUse,
    #[error("not a virtual disk file")]
    NotVhdx,
    #[error("path does not exist")]
    NotFound,
    #[error("path is not a folder")]
    NotAFolder,
    #[error("process exited with code {0}")]
    ExitCode(u32),
    #[error("windows error {0}")]
    Win32(u32),
    #[error("io error: {0}")]
    Io(String),
    #[error("cancelled")]
    Cancelled,
    #[error("file is in use by another program")]
    InUse,
    #[error("access denied")]
    AccessDenied,
    #[error("cloud only file, left alone")]
    CloudFile,
    #[error("folder is nested too deep")]
    TooDeep,
    #[error("the restore point could not be created")]
    RestorePointFailed,
}

impl CleanupError {
    pub fn code(&self) -> &'static str {
        match self {
            CleanupError::Blocked(r) => r.code(),
            CleanupError::Sandbox(s) => s.code(),
            CleanupError::PlanBlocked => "blocked",
            CleanupError::CommandNotAllowed => "command_not_allowed",
            CleanupError::TargetNotAllowed => "open_target_not_allowed",
            CleanupError::UacRefused => "uac_refused",
            CleanupError::VhdxInUse => "vhdx_in_use",
            CleanupError::NotVhdx => "not_vhdx",
            CleanupError::NotFound => "not_found",
            CleanupError::NotAFolder => "not_a_folder",
            CleanupError::ExitCode(_) => "command_failed",
            CleanupError::Win32(_) => "win32_error",
            CleanupError::Io(_) => "io_error",
            CleanupError::Cancelled => "cancelled",
            CleanupError::InUse => "in_use",
            CleanupError::AccessDenied => "access_denied",
            CleanupError::CloudFile => "cloud_only_skipped",
            CleanupError::TooDeep => "too_deep",
            CleanupError::RestorePointFailed => "restore_point_failed",
        }
    }

    pub fn to_api(&self) -> ApiError {
        match self {
            CleanupError::ExitCode(_) | CleanupError::Win32(_) | CleanupError::Io(_) => {
                ApiError::with_detail(self.code(), self.to_string())
            }
            _ => ApiError::new(self.code()),
        }
    }

    pub fn is_block(&self) -> bool {
        matches!(self, CleanupError::Blocked(_) | CleanupError::Sandbox(_) | CleanupError::PlanBlocked)
    }
}

impl From<std::io::Error> for CleanupError {
    fn from(e: std::io::Error) -> Self {
        CleanupError::Io(e.to_string())
    }
}
