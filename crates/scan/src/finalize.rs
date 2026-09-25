//! Turns a `TreeBuilder` into a finished `ScanTree`.
//!
//! Steps: child lists from parent links, hard link dedupe, totals bottom up, children sorted by
//! size, then an in place renumbering into depth first preorder. Nodes not reachable from the
//! root (excluded folders, orphan MFT records) are dropped.

use crate::builder::TreeBuilder;
use crate::names::NameInterner;
use crate::node::{flags, Node, NONE};
use crate::size::hardlink_dups;
use crate::tree::{ScanTree, TreeSummary};

/// Parent to children index (compressed sparse rows).
struct Children {
    offsets: Vec<u32>,
    kids: Vec<u32>,
}

impl Children {
    fn build(nodes: &[Node], root: u32) -> Self {
        let n = nodes.len();
        let mut offsets = vec![0u32; n + 1];
        let valid = |i: usize, p: u32| i as u32 != root && (p as usize) < n && p != i as u32;
        for (i, node) in nodes.iter().enumerate() {
            if valid(i, node.parent) {
                offsets[node.parent as usize + 1] += 1;
            }
        }
        for i in 0..n {
            offsets[i + 1] += offsets[i];
        }
        let mut cursor = offsets[..n].to_vec();
        let mut kids = vec![0u32; offsets[n] as usize];
        for (i, node) in nodes.iter().enumerate() {
            if valid(i, node.parent) {
                let c = &mut cursor[node.parent as usize];
                kids[*c as usize] = i as u32;
                *c += 1;
            }
        }
        Self { offsets, kids }
    }

    fn of(&self, i: u32) -> &[u32] {
        let (a, b) = (self.offsets[i as usize] as usize, self.offsets[i as usize + 1] as usize);
        &self.kids[a..b]
    }

    fn of_mut(&mut self, i: u32) -> &mut [u32] {
        let (a, b) = (self.offsets[i as usize] as usize, self.offsets[i as usize + 1] as usize);
        &mut self.kids[a..b]
    }
}

fn preorder(children: &Children, root: u32, cap: usize) -> Vec<u32> {
    let mut order = Vec::with_capacity(cap);
    let mut stack = vec![root];
    while let Some(i) = stack.pop() {
        order.push(i);
        stack.extend(children.of(i).iter().rev());
    }
    order
}

/// Rank for picking which hard link keeps the bytes: outside WinSxS first, then the shallowest
/// path, then the path itself so repeated scans agree.
fn link_rank(nodes: &[Node], names: &NameInterner, root: u32, id: u32) -> (bool, usize, String) {
    let mut parts = Vec::new();
    let mut in_winsxs = false;
    let mut cur = id;
    while cur != root && (cur as usize) < nodes.len() && parts.len() < 4096 {
        let name = names.get(nodes[cur as usize].name);
        if name.eq_ignore_ascii_case("winsxs") {
            in_winsxs = true;
        }
        parts.push(name);
        cur = nodes[cur as usize].parent;
    }
    parts.reverse();
    (in_winsxs, parts.len(), parts.join("\\").to_lowercase())
}

fn dedupe_hardlinks(b: &mut TreeBuilder, root: u32) {
    let reachable = |nodes: &[Node], id: u32| nodes.get(id as usize).is_some_and(|n| n.end == 1);
    let nodes = &b.nodes;
    b.ids64.retain(|&(_, id)| reachable(nodes, id));
    b.ids128.retain(|&(_, id)| reachable(nodes, id));
    let names = &b.names;
    let mut dups = hardlink_dups(&mut b.ids64, |id| link_rank(nodes, names, root, id));
    dups.extend(hardlink_dups(&mut b.ids128, |id| link_rank(nodes, names, root, id)));
    for d in dups {
        let n = &mut b.nodes[d as usize];
        n.own_size = 0;
        n.flags |= flags::HARDLINK_DUP;
    }
    b.ids64 = Vec::new();
    b.ids128 = Vec::new();
}

fn sum_bottom_up(nodes: &mut [Node], order: &[u32], children: &Children, root: u32) {
    for &i in order {
        let child_count = children.of(i).len() as u32;
        let n = &mut nodes[i as usize];
        n.total_size = n.own_size;
        n.file_count = u32::from(!n.is_dir());
        n.dir_count = 0;
        n.child_count = child_count;
        // `end` holds the subtree node count until renumbering.
        n.end = 1;
    }
    for &i in order.iter().rev() {
        if i == root {
            continue;
        }
        let c = nodes[i as usize];
        let p = &mut nodes[c.parent as usize];
        p.total_size += c.total_size;
        p.file_count += c.file_count;
        p.dir_count += c.dir_count + u32::from(c.is_dir());
        p.end += c.end;
        p.modified = p.modified.max(c.modified);
    }
}

fn sort_children(nodes: &[Node], names: &NameInterner, children: &mut Children, order: &[u32]) {
    for &i in order {
        let kids = children.of_mut(i);
        if kids.len() > 1 {
            kids.sort_unstable_by(|&a, &b| {
                let (na, nb) = (&nodes[a as usize], &nodes[b as usize]);
                nb.total_size.cmp(&na.total_size).then_with(|| names.get(na.name).cmp(names.get(nb.name)))
            });
        }
    }
}

/// Applies `new_id` as a permutation: the node at `i` moves to `new_id[i]`.
fn permute(nodes: &mut [Node], new_id: &mut [u32]) {
    for i in 0..nodes.len() {
        while new_id[i] as usize != i {
            let j = new_id[i] as usize;
            nodes.swap(i, j);
            new_id.swap(i, j);
        }
    }
}

pub fn finalize(mut b: TreeBuilder, root: u32, root_path: String) -> ScanTree {
    let n = b.nodes.len();
    let root = if (root as usize) < n { root } else { 0 };
    let mut children = Children::build(&b.nodes, root);
    let order = preorder(&children, root, n);

    // Mark reachable nodes (end == 1) before dedupe so unreachable links never win.
    for node in b.nodes.iter_mut() {
        node.end = 0;
    }
    for &i in &order {
        b.nodes[i as usize].end = 1;
    }
    dedupe_hardlinks(&mut b, root);

    sum_bottom_up(&mut b.nodes, &order, &children, root);
    sort_children(&b.nodes, &b.names, &mut children, &order);
    let reachable = order.len();
    drop(order);

    // Final preorder numbering with sorted children.
    let mut new_id = vec![NONE; n];
    let mut next = 0u32;
    let mut stack = vec![root];
    while let Some(i) = stack.pop() {
        new_id[i as usize] = next;
        next += 1;
        stack.extend(children.of(i).iter().rev());
    }
    drop(children);
    for slot in new_id.iter_mut().filter(|s| **s == NONE) {
        *slot = next;
        next += 1;
    }
    for i in 0..n {
        let id = new_id[i];
        if id as usize >= reachable {
            continue;
        }
        let node = &mut b.nodes[i];
        node.end += id;
        node.parent = if i as u32 == root { NONE } else { new_id[node.parent as usize] };
    }
    permute(&mut b.nodes, &mut new_id);
    drop(new_id);
    b.nodes.truncate(reachable);
    b.nodes.shrink_to_fit();

    let nodes = b.nodes;
    let summary = TreeSummary::compute(&nodes);
    ScanTree::from_parts(nodes, b.names.finish(), root_path, summary)
}
