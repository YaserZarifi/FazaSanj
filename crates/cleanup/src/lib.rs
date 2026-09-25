//! Cleanup plans, dry runs and executors.

mod commands;
mod error;
mod guard;
mod win;

pub use error::CleanupError;
pub use commands::{CommandSpec, KnownCommand, OpenTarget};
