//! Cleanup plans, dry runs and executors.

mod commands;
mod error;
mod guard;
mod recycle_bin;
mod win;

pub use error::CleanupError;
pub use commands::{CommandSpec, KnownCommand, OpenTarget};
pub use recycle_bin::{parse_index, recycle_bin_lookup, RecycledItem};
