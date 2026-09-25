//! Things the knowledge base does not know: orphaned app data, stale files, duplicates and old
//! projects. Every finding has a confidence and is never marked `safe`.

mod installed;
mod names;
mod orphans;
mod util;

pub use installed::installed_programs;
pub use orphans::{find_orphans, find_orphans_at};

/// A folder from the scan tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirRecord {
    pub node_id: Option<u32>,
    pub path: String,
    pub name: String,
    pub size: u64,
    /// Newest modification time in the subtree (unix ms).
    pub modified: Option<i64>,
}

/// A file from the scan tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRecord {
    pub node_id: Option<u32>,
    pub path: String,
    pub size: u64,
    pub modified: Option<i64>,
    pub accessed: Option<i64>,
}

/// One entry of the registry Uninstall keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledApp {
    pub display_name: String,
    pub publisher: Option<String>,
    pub install_location: Option<String>,
    /// Hidden from Apps and Features (runtimes, parts of bigger products).
    pub system_component: bool,
}

impl InstalledApp {
    pub fn new(display_name: &str, publisher: Option<&str>, install_location: Option<&str>) -> Self {
        Self {
            display_name: display_name.to_string(),
            publisher: publisher.map(str::to_string),
            install_location: install_location.map(str::to_string),
            system_component: false,
        }
    }
}
