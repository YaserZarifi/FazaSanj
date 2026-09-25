//! Tree under construction. Both scanners push nodes here, `finalize` turns it into a `ScanTree`.

use crate::names::NameInterner;
use crate::node::{flags, Node, NONE, NO_TIME};

pub struct TreeBuilder {
    pub nodes: Vec<Node>,
    pub names: NameInterner,
    /// Hard link candidates keyed by file id. NTFS ids fit in 64 bits, which halves the memory.
    pub ids64: Vec<(u64, u32)>,
    pub ids128: Vec<(u128, u32)>,
}

impl Default for TreeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TreeBuilder {
    /// Starts with the root folder as node 0.
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    pub fn with_capacity(cap: usize) -> Self {
        let mut nodes = Vec::with_capacity(cap);
        nodes.push(Node::new(NONE, 0, 0, NO_TIME, flags::DIR));
        Self { nodes, names: NameInterner::new(), ids64: Vec::new(), ids128: Vec::new() }
    }

    pub fn push(&mut self, parent: u32, name: &str, own_size: u64, modified: i64, flags: u16) -> u32 {
        let id = self.nodes.len() as u32;
        let name = self.names.intern(name);
        self.nodes.push(Node::new(parent, name, own_size, modified, flags));
        id
    }

    /// Remembers a file id so hard links to the same file are counted once.
    pub fn add_link_candidate(&mut self, file_id: u128, node: u32) {
        match u64::try_from(file_id) {
            Ok(small) => self.ids64.push((small, node)),
            Err(_) => self.ids128.push((file_id, node)),
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}
