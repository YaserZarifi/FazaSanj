//! The finished scan tree and its basic queries.
//!
//! Layout: nodes in depth first preorder, children sorted by size (largest first). The
//! subtree of node `n` is the id range `n..end`, the first child of a folder is `n + 1`,
//! and the next sibling of `c` is `c.end`.

use std::sync::OnceLock;

use fazasanj_model::{ChildSort, ChildrenPage, NodeInfo, TypeGroup};

use crate::names::Names;
use crate::node::{flags, Node, NodeId, NONE};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TreeSummary {
    pub total_bytes: u64,
    pub files: u64,
    pub dirs: u64,
    /// Files that live only in the cloud (counted as 0 bytes).
    pub cloud_only: u64,
    /// Folders we could not read.
    pub access_denied: u64,
    /// Extra hard links that were not counted again.
    pub hardlink_dups: u64,
}

impl TreeSummary {
    pub(crate) fn compute(nodes: &[Node]) -> Self {
        let mut s = TreeSummary::default();
        if let Some(root) = nodes.first() {
            s.total_bytes = root.total_size;
            s.files = u64::from(root.file_count);
            s.dirs = u64::from(root.dir_count);
        }
        for n in nodes {
            s.cloud_only += u64::from(n.has(flags::CLOUD_ONLY) && !n.is_dir());
            s.access_denied += u64::from(n.has(flags::ACCESS_DENIED));
            s.hardlink_dups += u64::from(n.has(flags::HARDLINK_DUP));
        }
        s
    }
}

pub struct ScanTree {
    pub(crate) nodes: Vec<Node>,
    pub(crate) names: Names,
    pub(crate) root_path: String,
    pub(crate) summary: TreeSummary,
    pub(crate) type_cache: OnceLock<Vec<TypeGroup>>,
}

impl std::fmt::Debug for ScanTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScanTree").field("root_path", &self.root_path).field("summary", &self.summary).finish()
    }
}

/// Iterator over the direct children of a folder, largest first.
pub struct Children<'a> {
    nodes: &'a [Node],
    next: u32,
    end: u32,
}

impl Iterator for Children<'_> {
    type Item = NodeId;
    fn next(&mut self) -> Option<NodeId> {
        if self.next >= self.end {
            return None;
        }
        let id = self.next;
        self.next = self.nodes.get(id as usize).map_or(self.end, |n| n.end.max(id + 1));
        Some(id)
    }
}

impl ScanTree {
    pub(crate) fn from_parts(nodes: Vec<Node>, names: Names, root_path: String, summary: TreeSummary) -> Self {
        Self { nodes, names, root_path, summary, type_cache: OnceLock::new() }
    }

    pub fn root(&self) -> NodeId {
        0
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// The scanned folder, like `C:\` or `D:\Projects`.
    pub fn root_path(&self) -> &str {
        &self.root_path
    }

    pub fn summary(&self) -> TreeSummary {
        self.summary
    }

    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id as usize)
    }

    pub fn name(&self, id: NodeId) -> &str {
        match self.node(id) {
            Some(_) if id == 0 => &self.root_path,
            Some(n) => self.names.get(n.name),
            None => "",
        }
    }

    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.node(id).map(|n| n.parent).filter(|&p| p != NONE)
    }

    pub fn children(&self, id: NodeId) -> Children<'_> {
        match self.node(id) {
            Some(n) if n.child_count > 0 => Children { nodes: &self.nodes, next: id + 1, end: n.end },
            _ => Children { nodes: &self.nodes, next: 0, end: 0 },
        }
    }

    /// Rough heap use of the tree, for diagnostics.
    pub fn memory_bytes(&self) -> usize {
        self.nodes.capacity() * std::mem::size_of::<Node>() + self.names.heap_bytes() + self.root_path.capacity()
    }

    /// Full path of a node.
    pub fn path_of(&self, id: NodeId) -> String {
        if self.node(id).is_none() {
            return String::new();
        }
        let mut parts = Vec::new();
        let mut cur = id;
        while cur != 0 && cur != NONE {
            let Some(n) = self.node(cur) else { break };
            parts.push(self.names.get(n.name));
            cur = n.parent;
        }
        let mut out = self.root_path.clone();
        for p in parts.iter().rev() {
            if !out.ends_with('\\') {
                out.push('\\');
            }
            out.push_str(p);
        }
        out
    }

    /// Looks up a node by full path, case insensitive. Accepts `/` too.
    pub fn find_by_path(&self, path: &str) -> Option<NodeId> {
        let path = path.replace('/', "\\");
        let root = self.root_path.trim_end_matches('\\');
        let path_trim = path.trim_end_matches('\\');
        let rest = strip_prefix_ci(path_trim, root)?;
        if !rest.is_empty() && !rest.starts_with('\\') {
            return None;
        }
        let mut cur = 0;
        for part in rest.split('\\').filter(|p| !p.is_empty()) {
            cur = self.children(cur).find(|&c| eq_ci(self.names.get(self.nodes[c as usize].name), part))?;
        }
        Some(cur)
    }

    /// Everything the UI shows for one node. `explanation` is filled by the app.
    pub fn node_info(&self, id: NodeId) -> Option<NodeInfo> {
        let n = self.node(id)?;
        Some(NodeInfo {
            id,
            parent: self.parent(id),
            name: self.name(id).to_string(),
            path: self.path_of(id),
            is_dir: n.is_dir(),
            size: n.total_size,
            file_count: u64::from(n.file_count),
            dir_count: u64::from(n.dir_count),
            child_count: n.child_count,
            modified: n.modified(),
            flags: n.model_flags(),
            category: self.effective_category(id),
            explanation: None,
        })
    }

    /// One page of children. Size order is free (stored order), the others sort on demand.
    pub fn children_page(&self, id: NodeId, sort: ChildSort, offset: u32, limit: u32) -> Option<ChildrenPage> {
        let n = self.node(id)?;
        let total = n.child_count;
        let ids: Vec<NodeId> = match sort {
            ChildSort::Size => self.children(id).skip(offset as usize).take(limit as usize).collect(),
            ChildSort::Name => {
                let mut all: Vec<NodeId> = self.children(id).collect();
                all.sort_by_cached_key(|&c| self.name(c).to_lowercase());
                all.into_iter().skip(offset as usize).take(limit as usize).collect()
            }
            ChildSort::Modified => {
                let mut all: Vec<NodeId> = self.children(id).collect();
                all.sort_by_key(|&c| std::cmp::Reverse(self.nodes[c as usize].modified));
                all.into_iter().skip(offset as usize).take(limit as usize).collect()
            }
        };
        let items = ids.into_iter().filter_map(|c| self.node_info(c)).collect();
        Some(ChildrenPage { total, offset, items })
    }
}

pub(crate) fn eq_ci(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b) || (!a.is_ascii() && a.to_lowercase() == b.to_lowercase())
}

fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    let head = s.get(..prefix.len())?;
    eq_ci(head, prefix).then(|| &s[prefix.len()..])
}
