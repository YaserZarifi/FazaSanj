//! Scan tree, size logic, normal walker and the fast scan client.

mod builder;
mod category;
mod finalize;
mod guess;
mod names;
mod node;
mod options;
mod progress;
mod queries;
mod rootpath;
pub mod size;
mod tree;
mod types;
mod visit;

#[cfg(test)]
mod tree_tests;

pub use builder::TreeBuilder;
pub use finalize::finalize;
pub use node::{flags, Node, NodeId};
pub use options::{ProgressFn, ProgressSnapshot, ScanError, ScanOptions, ScanOutcome};
pub use tree::{Children, ScanTree, TreeSummary};
pub use types::{extension_of, group_of};
pub use visit::{FileItem, Visit, VisitEntry};
