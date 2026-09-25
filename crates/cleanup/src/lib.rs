//! Cleanup plans, dry runs and executors.
//!
//! The hard block list and the debug sandbox are enforced here, inside the engine, right before
//! every path is touched. Files in use are skipped, never forced. Links are never followed.

mod commands;
mod engine;
mod error;
mod exec;
mod guard;
mod plan;
mod recycle_bin;
mod remove;
mod walk;
mod win;

pub use commands::{CommandSpec, KnownCommand, OpenTarget};
pub use engine::CleanupEngine;
pub use error::CleanupError;
pub use plan::ActionExtras;
pub use recycle_bin::{parse_index, recycle_bin_lookup, RecycledItem};

/// Creates a system restore point through an elevated `Checkpoint-Computer`.
pub fn create_restore_point(description: &str) -> Result<(), CleanupError> {
    exec::create_restore_point(description)
}
