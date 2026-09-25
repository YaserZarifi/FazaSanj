//! Scan tree, size logic, normal walker and the fast scan client.

mod builder;
mod category;
mod compare;
mod fast;
mod finalize;
mod guess;
mod ingest;
mod names;
mod node;
mod options;
mod progress;
mod queries;
mod rootpath;
mod run;
pub mod size;
mod tree;
mod types;
mod visit;
mod walker;
pub mod wire;

#[cfg(test)]
mod tree_tests;

pub use builder::TreeBuilder;
pub use compare::{compare_scanners, diff_trees};
pub use finalize::finalize;
pub use node::{flags, Node, NodeId};
pub use options::{ProgressFn, ProgressSnapshot, ScanError, ScanOptions, ScanOutcome};
pub use rootpath::display_path;
pub use run::run_scan;
pub use tree::{Children, ScanTree, TreeSummary};
pub use types::{extension_of, group_of};
pub use visit::{FileItem, Visit, VisitEntry};
