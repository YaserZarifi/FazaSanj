//! Rule tags on nodes and category roll ups.
//!
//! A node's category is its own tag, else the nearest tagged ancestor's, else the built-in guess.

use fazasanj_model::Category;

use crate::guess::{guess_dir, guess_file};
use crate::node::{category_to_u8, NodeId};
use crate::tree::ScanTree;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CatCtx {
    pub cat: Category,
    pub from_tag: bool,
}

fn lower<'a>(s: &'a str, buf: &'a mut String) -> &'a str {
    buf.clear();
    if s.is_ascii() {
        buf.extend(s.chars().map(|c| c.to_ascii_lowercase()));
    } else {
        buf.push_str(&s.to_lowercase());
    }
    buf.as_str()
}

/// Components of the root path below the drive, like `["windows", "system32"]` for
/// `C:\Windows\System32`. `None` when the root is not on a drive letter.
fn root_components(root: &str) -> Option<Vec<String>> {
    let b = root.as_bytes();
    if b.len() < 2 || !b[0].is_ascii_alphabetic() || b[1] != b':' {
        return None;
    }
    Some(root[2..].split('\\').filter(|p| !p.is_empty()).map(str::to_lowercase).collect())
}

impl ScanTree {
    fn root_drive_depth(&self) -> Option<u32> {
        root_components(&self.root_path).map(|c| c.len() as u32)
    }

    pub(crate) fn root_ctx(&self) -> CatCtx {
        if let Some((cat, _)) = self.nodes.first().and_then(|n| n.tag()) {
            return CatCtx { cat, from_tag: true };
        }
        let mut cat = Category::Unknown;
        if let Some(parts) = root_components(&self.root_path) {
            for (i, p) in parts.iter().enumerate() {
                cat = guess_dir(p, Some(i as u32 + 1), cat);
            }
        }
        CatCtx { cat, from_tag: false }
    }

    /// Category of `id` given its parent's context and its depth below the scan root.
    pub(crate) fn child_ctx(&self, parent: CatCtx, id: NodeId, depth: u32, buf: &mut String) -> CatCtx {
        let Some(n) = self.node(id) else { return parent };
        if let Some((cat, _)) = n.tag() {
            return CatCtx { cat, from_tag: true };
        }
        if parent.from_tag {
            return parent;
        }
        let name = lower(self.names.get(n.name), buf);
        let cat = if n.is_dir() {
            guess_dir(name, self.root_drive_depth().map(|d| d + depth), parent.cat)
        } else {
            guess_file(name, parent.cat)
        };
        CatCtx { cat, from_tag: false }
    }

    /// Category shown for a node: own tag, nearest tagged ancestor, or the built-in guess.
    pub fn effective_category(&self, id: NodeId) -> Category {
        if self.node(id).is_none() {
            return Category::Unknown;
        }
        let mut chain = Vec::new();
        let mut cur = id;
        while cur != 0 {
            chain.push(cur);
            match self.parent(cur) {
                Some(p) => cur = p,
                None => break,
            }
        }
        let mut ctx = self.root_ctx();
        let mut buf = String::new();
        for (depth, &c) in chain.iter().rev().enumerate() {
            ctx = self.child_ctx(ctx, c, depth as u32 + 1, &mut buf);
        }
        ctx.cat
    }

    /// Attaches a knowledge base result to a node. Returns false for an unknown id.
    pub fn set_tag(&mut self, id: NodeId, category: Category, rule: Option<u32>) -> bool {
        match self.nodes.get_mut(id as usize) {
            Some(n) => {
                n.set_tag(category, rule);
                true
            }
            None => false,
        }
    }

    /// Many `set_tag` calls at once, for the rules engine.
    pub fn set_tags(&mut self, tags: impl IntoIterator<Item = (NodeId, Category, Option<u32>)>) {
        for (id, cat, rule) in tags {
            self.set_tag(id, cat, rule);
        }
    }

    pub fn clear_tags(&mut self) {
        for n in &mut self.nodes {
            n.clear_tag();
        }
    }

    pub fn tag_of(&self, id: NodeId) -> Option<(Category, Option<u32>)> {
        self.node(id)?.tag()
    }

    /// Tagged nodes that have no tagged ancestor, largest first within each folder.
    pub fn tagged_roots(&self) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut id = 0u32;
        let n = self.nodes.len() as u32;
        while id < n {
            let node = &self.nodes[id as usize];
            if node.tag().is_some() {
                out.push(id);
                id = node.end.max(id + 1);
            } else {
                id += 1;
            }
        }
        out
    }

    /// Bytes per category, each byte counted once, largest first. Zero categories are left out.
    pub fn category_totals(&self) -> Vec<(Category, u64)> {
        let mut totals = [0u64; 11];
        let mut stack: Vec<(u32, CatCtx)> = Vec::with_capacity(64);
        let root = self.root_ctx();
        let mut buf = String::new();
        for id in 1..self.nodes.len() as u32 {
            while stack.last().is_some_and(|&(end, _)| id >= end) {
                stack.pop();
            }
            let parent = stack.last().map_or(root, |&(_, c)| c);
            let depth = stack.len() as u32 + 1;
            let ctx = self.child_ctx(parent, id, depth, &mut buf);
            let node = &self.nodes[id as usize];
            if node.is_dir() {
                stack.push((node.end, ctx));
            } else {
                totals[category_to_u8(ctx.cat) as usize % 11] += node.own_size;
            }
        }
        let mut out: Vec<(Category, u64)> = crate::node::all_categories()
            .iter()
            .map(|&c| (c, totals[category_to_u8(c) as usize % 11]))
            .filter(|&(_, b)| b > 0)
            .collect();
        out.sort_by(|a, b| b.1.cmp(&a.1));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::root_components;

    #[test]
    fn root_parts() {
        assert_eq!(root_components(r"C:\"), Some(vec![]));
        assert_eq!(root_components(r"C:\Windows\System32"), Some(vec!["windows".into(), "system32".into()]));
        assert_eq!(root_components(r"\\srv\share"), None);
    }
}
