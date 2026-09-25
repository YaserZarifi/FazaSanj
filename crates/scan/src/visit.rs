//! Top down walks over the finished tree that build paths as they go, so walking 2 million
//! nodes costs one string append per node instead of a parent chain lookup.

use crate::node::NodeId;
use crate::tree::ScanTree;

/// What to do after visiting a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visit {
    Continue,
    /// Do not go into this folder.
    SkipChildren,
    Stop,
}

/// One node as seen by `visit_dirs_and_files`.
#[derive(Debug, Clone, Copy)]
pub struct VisitEntry<'a> {
    pub id: NodeId,
    pub name: &'a str,
    /// Full path in original case.
    pub path: &'a str,
    /// Full path, lowercase, for case insensitive matching.
    pub path_lower: &'a str,
    pub is_dir: bool,
    /// Total size of the subtree.
    pub size: u64,
    /// Unix ms (newest in subtree for folders).
    pub modified: Option<i64>,
    /// 0 for the root.
    pub depth: u32,
}

/// A file as returned by `ScanTree::files`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileItem {
    pub id: NodeId,
    pub path: String,
    pub size: u64,
    pub modified: Option<i64>,
    /// Another link of this file was counted instead (size is 0 here).
    pub hardlink_dup: bool,
}

fn push_lower(dst: &mut String, s: &str) {
    if s.is_ascii() {
        dst.extend(s.chars().map(|c| c.to_ascii_lowercase()));
    } else {
        dst.push_str(&s.to_lowercase());
    }
}

/// Keeps the current path while ids advance in preorder.
pub(crate) struct PathCursor {
    /// (subtree end, path len, lower len) of each open folder.
    stack: Vec<(u32, usize, usize)>,
    pub path: String,
    pub lower: String,
    with_lower: bool,
    root_lower_len: usize,
}

impl PathCursor {
    pub fn new(tree: &ScanTree, with_lower: bool) -> Self {
        let root = tree.root_path();
        let mut lower = String::new();
        if with_lower {
            push_lower(&mut lower, root);
        }
        let root_lower_len = lower.len();
        Self { stack: Vec::with_capacity(64), path: root.to_string(), lower, with_lower, root_lower_len }
    }

    /// Depth of the node last entered (root is 0).
    pub fn depth(&self) -> u32 {
        self.stack.len() as u32
    }

    /// Moves to node `id`, which must come after the previous one in preorder.
    /// Afterwards `path` is its full path and `depth()` its depth.
    pub fn enter(&mut self, tree: &ScanTree, id: NodeId) {
        if id == 0 {
            return;
        }
        while let Some(&(end, _, _)) = self.stack.last() {
            if id < end {
                break;
            }
            self.stack.pop();
        }
        let root_len = tree.root_path().len();
        let (plen, llen) = self.stack.last().map_or((root_len, self.root_lower_len), |&(_, p, l)| (p, l));
        self.path.truncate(plen);
        self.lower.truncate(llen);
        let name = tree.name(id);
        if !self.path.ends_with('\\') {
            self.path.push('\\');
            if self.with_lower {
                self.lower.push('\\');
            }
        }
        self.path.push_str(name);
        if self.with_lower {
            push_lower(&mut self.lower, name);
        }
        if let Some(n) = tree.node(id) {
            if n.is_dir() {
                self.stack.push((n.end, self.path.len(), self.lower.len()));
            }
        }
    }

    /// Depth of `id` right after `enter(id)`.
    pub fn depth_of_entered(&self, tree: &ScanTree, id: NodeId) -> u32 {
        let is_dir = tree.node(id).is_some_and(|n| n.is_dir());
        if id == 0 {
            0
        } else if is_dir {
            self.depth()
        } else {
            self.depth() + 1
        }
    }
}

impl ScanTree {
    /// Visits every node top down (root first), with full paths. Return `SkipChildren` to not
    /// go into a folder, `Stop` to end the walk.
    pub fn visit_dirs_and_files(&self, f: &mut impl FnMut(&VisitEntry) -> Visit) {
        let mut cur = PathCursor::new(self, true);
        let mut id = 0u32;
        let n = self.nodes.len() as u32;
        while id < n {
            cur.enter(self, id);
            let node = &self.nodes[id as usize];
            let entry = VisitEntry {
                id,
                name: if id == 0 { "" } else { self.names.get(node.name) },
                path: &cur.path,
                path_lower: &cur.lower,
                is_dir: node.is_dir(),
                size: node.total_size,
                modified: node.modified(),
                depth: cur.depth_of_entered(self, id),
            };
            match f(&entry) {
                Visit::Continue => id += 1,
                Visit::SkipChildren => id = node.end.max(id + 1),
                Visit::Stop => break,
            }
        }
    }

    /// Every file with its full path, for heuristics.
    pub fn files(&self) -> impl Iterator<Item = FileItem> + '_ {
        let mut cur = PathCursor::new(self, false);
        let n = self.nodes.len() as u32;
        (1..n).filter_map(move |id| {
            let node = &self.nodes[id as usize];
            if node.is_dir() {
                // Folders still move the cursor so child paths are right.
                cur.enter(self, id);
                return None;
            }
            cur.enter(self, id);
            Some(FileItem {
                id,
                path: cur.path.clone(),
                size: node.own_size,
                modified: node.modified(),
                hardlink_dup: node.has(crate::node::flags::HARDLINK_DUP),
            })
        })
    }

    /// Folder paths with size and file count down to `max_depth` (root is depth 0).
    /// Used for compact snapshots and for comparing scanners.
    pub fn folder_sizes(&self, max_depth: u32) -> Vec<(String, u64, u64)> {
        let mut out = Vec::new();
        self.visit_dirs_and_files(&mut |e| {
            if !e.is_dir {
                return Visit::Continue;
            }
            out.push((e.path.to_string(), e.size, self.nodes[e.id as usize].file_count as u64));
            if e.depth >= max_depth {
                Visit::SkipChildren
            } else {
                Visit::Continue
            }
        });
        out
    }
}
