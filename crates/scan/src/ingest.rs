//! Rebuilds the arena from MFT records sent by the helper. Pure logic, tested without a volume.
//!
//! Records arrive in MFT order, so a parent can come after its children. Parents are stored
//! as record numbers first and resolved once everything is in. Several records with the same
//! record number are hard links of one file and go through the usual dedupe in `finalize`.

use crate::builder::TreeBuilder;
use crate::finalize::finalize;
use crate::node::{flags, NONE};
use crate::size::{attribute_flags, file_size};
use crate::tree::{eq_ci, ScanTree};
use crate::wire::{WireRecord, FLAG_DIR, ROOT_RECORD};

pub struct Ingest {
    b: TreeBuilder,
    /// First node created for each record number.
    first_node: Vec<u32>,
    /// Parent record number of node `i + 1` (node 0 is the root).
    parent_rec: Vec<u64>,
}

/// What one record added, for progress counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Added {
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
}

impl Default for Ingest {
    fn default() -> Self {
        Self::new()
    }
}

impl Ingest {
    pub fn new() -> Self {
        let mut first_node = vec![NONE; 1 << 16];
        first_node[ROOT_RECORD as usize] = 0;
        Self { b: TreeBuilder::with_capacity(1 << 16), first_node, parent_rec: Vec::with_capacity(1 << 16) }
    }

    pub fn add(&mut self, r: &WireRecord) -> Added {
        if r.rec == ROOT_RECORD {
            self.b.nodes[0].modified = r.modified;
            return Added::default();
        }
        let Ok(rec) = usize::try_from(r.rec) else { return Added::default() };
        if rec >= self.first_node.len() {
            let want = (rec + 1).next_power_of_two();
            self.first_node.resize(want, NONE);
        }
        let is_dir = r.flags & FLAG_DIR != 0;
        let (size, f) = if is_dir {
            (0, attribute_flags(r.attributes) | flags::DIR)
        } else {
            // The helper already summed the real allocated bytes of all data streams.
            file_size(r.attributes, 0, r.size, None)
        };
        let node = self.b.push(NONE, r.name, size, r.modified, f);
        self.parent_rec.push(r.parent);
        let first = self.first_node[rec];
        if first == NONE {
            self.first_node[rec] = node;
        } else if !is_dir && size > 0 {
            self.b.add_link_candidate(u128::from(r.rec), first);
            self.b.add_link_candidate(u128::from(r.rec), node);
        }
        if is_dir {
            Added { dirs: 1, ..Added::default() }
        } else {
            Added { files: 1, bytes: size, ..Added::default() }
        }
    }

    /// Resolves parents and builds the tree. `sub_path` are the folder names from the volume
    /// root down to the requested scan root (empty for a whole drive), `excluded` likewise for
    /// each excluded folder. Returns `None` if the scan root is not in the MFT.
    pub fn finish(mut self, sub_path: &[String], excluded: &[Vec<String>], display: String) -> Option<ScanTree> {
        for (i, &p) in self.parent_rec.iter().enumerate() {
            let parent = usize::try_from(p).ok().and_then(|p| self.first_node.get(p).copied()).unwrap_or(NONE);
            self.b.nodes[i + 1].parent = parent;
        }
        self.parent_rec = Vec::new();
        self.first_node = Vec::new();

        for ex in excluded {
            if let Some(id) = self.find(ex) {
                if id != 0 {
                    self.b.nodes[id as usize].parent = NONE;
                }
            }
        }
        let root = self.find(sub_path)?;
        if !self.b.nodes[root as usize].is_dir() {
            return None;
        }
        Some(finalize(self.b, root, display))
    }

    /// Finds a node by names from the volume root. Linear per level, only used a few times.
    fn find(&self, parts: &[String]) -> Option<u32> {
        let mut cur = 0u32;
        for part in parts {
            let names = &self.b.names;
            cur = self
                .b
                .nodes
                .iter()
                .enumerate()
                .skip(1)
                .find(|(_, n)| n.parent == cur && eq_ci(names.get(n.name), part))
                .map(|(i, _)| i as u32)?;
        }
        Some(cur)
    }
}

/// Folder names below the drive for a display path like `C:\Users\me`.
pub fn components_below_drive(display: &str) -> Vec<String> {
    display.get(2..).unwrap_or("").split('\\').filter(|p| !p.is_empty()).map(str::to_string).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(rec: u64, parent: u64, name: &str, size: u64, dir: bool) -> WireRecord<'_> {
        WireRecord {
            rec,
            parent,
            size,
            modified: rec as i64,
            attributes: if dir { 0x10 } else { 0x20 },
            flags: if dir { FLAG_DIR } else { 0 },
            name,
        }
    }

    fn sample() -> Ingest {
        let mut i = Ingest::new();
        // Children before parents, like the real MFT order can be.
        i.add(&rec(100, 40, "kernel.dll", 4096, false));
        i.add(&rec(100, 41, "kernel.dll", 4096, false));
        i.add(&rec(101, 40, "big.bin", 1 << 20, false));
        i.add(&rec(40, 30, "System32", 0, true));
        i.add(&rec(30, 5, "Windows", 0, true));
        i.add(&rec(42, 30, "WinSxS", 0, true));
        i.add(&rec(41, 42, "amd64_x", 0, true));
        i.add(&rec(5, 5, ".", 0, true));
        // Placeholder: not on disk, counts as 0.
        i.add(&WireRecord { attributes: 0x0040_0000, ..rec(102, 5, "cloud.docx", 8192, false) });
        // Orphan: parent record never sent.
        i.add(&rec(103, 9999, "lost.bin", 777, false));
        i
    }

    #[test]
    fn rebuilds_tree_with_dedupe() {
        let t = sample().finish(&[], &[], r"C:\".into()).expect("tree");
        let s = t.summary();
        assert_eq!(s.total_bytes, 4096 + (1 << 20));
        assert_eq!(s.files, 4);
        assert_eq!(s.dirs, 4);
        assert_eq!(s.hardlink_dups, 1);
        assert_eq!(s.cloud_only, 1);
        let sxs = t.find_by_path(r"C:\Windows\WinSxS").expect("winsxs");
        assert_eq!(t.node(sxs).map(|n| n.total_size), Some(0));
        assert!(t.find_by_path(r"C:\lost.bin").is_none());
        assert_eq!(t.node(0).and_then(|n| n.modified()), Some(102));
    }

    #[test]
    fn sub_root_and_exclusions() {
        let parts = components_below_drive(r"C:\Windows");
        let ex = vec![components_below_drive(r"C:\Windows\WinSxS")];
        let t = sample().finish(&parts, &ex, r"C:\Windows".into()).expect("tree");
        assert_eq!(t.root_path(), r"C:\Windows");
        assert!(t.find_by_path(r"C:\Windows\WinSxS").is_none());
        assert_eq!(t.summary().hardlink_dups, 0);
        assert_eq!(t.summary().total_bytes, 4096 + (1 << 20));
        assert!(sample().finish(&["Nope".into()], &[], "C:\\Nope".into()).is_none());
    }
}
