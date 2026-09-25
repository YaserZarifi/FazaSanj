//! Treemap slices and largest files.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use fazasanj_model::{FileEntry, TreemapNode};

use crate::category::CatCtx;
use crate::node::{flags, NodeId};
use crate::tree::ScanTree;

/// Items smaller than this share of their parent go into the "other" bucket.
const MIN_SHARE: f64 = 0.002;

impl ScanTree {
    /// A nested slice for the treemap: `depth` levels below `id`, at most `max_items` per level.
    /// Small leftovers are merged into one `is_other` item whose id is the parent's id.
    pub fn treemap(&self, id: NodeId, depth: u32, max_items: u32) -> Option<TreemapNode> {
        self.node(id)?;
        let ctx = CatCtx { cat: self.effective_category(id), from_tag: self.has_tag_in_chain(id) };
        let mut buf = String::new();
        Some(self.treemap_node(id, ctx, self.depth_of(id), depth, max_items.max(1) as usize, &mut buf))
    }

    fn depth_of(&self, id: NodeId) -> u32 {
        let mut d = 0;
        let mut cur = id;
        while let Some(p) = self.parent(cur) {
            d += 1;
            cur = p;
        }
        d
    }

    fn has_tag_in_chain(&self, id: NodeId) -> bool {
        let mut cur = Some(id);
        while let Some(c) = cur {
            if self.tag_of(c).is_some() {
                return true;
            }
            cur = self.parent(c);
        }
        false
    }

    fn treemap_node(
        &self,
        id: NodeId,
        ctx: CatCtx,
        abs_depth: u32,
        levels: u32,
        max_items: usize,
        buf: &mut String,
    ) -> TreemapNode {
        let n = &self.nodes[id as usize];
        let mut children = Vec::new();
        if levels > 0 && n.is_dir() {
            let min = (n.total_size as f64 * MIN_SHARE) as u64;
            let sized: Vec<NodeId> = self.children(id).filter(|&c| self.nodes[c as usize].total_size > 0).collect();
            let mut keep = sized.len();
            if keep > max_items {
                keep = max_items.saturating_sub(1);
            }
            // Children are sorted by size, so the first tiny one ends the kept list.
            if let Some(first_tiny) = sized[..keep].iter().position(|&c| self.nodes[c as usize].total_size < min) {
                // One tiny item alone is not worth an "other" bucket.
                if sized.len() - first_tiny > 1 {
                    keep = first_tiny;
                }
            }
            for &c in &sized[..keep] {
                let cctx = self.child_ctx(ctx, c, abs_depth + 1, buf);
                children.push(self.treemap_node(c, cctx, abs_depth + 1, levels - 1, max_items, buf));
            }
            let rest = &sized[keep..];
            if !rest.is_empty() {
                let (mut size, mut files, mut modified) = (0u64, 0u64, None::<i64>);
                for &c in rest {
                    let cn = &self.nodes[c as usize];
                    size += cn.total_size;
                    files += u64::from(cn.file_count);
                    modified = modified.max(cn.modified());
                }
                children.push(TreemapNode {
                    id,
                    name: String::new(),
                    size,
                    file_count: files,
                    modified,
                    category: ctx.cat,
                    is_dir: false,
                    is_other: true,
                    children: Vec::new(),
                });
            }
        }
        TreemapNode {
            id,
            name: self.name(id).to_string(),
            size: n.total_size,
            file_count: u64::from(n.file_count),
            modified: n.modified(),
            category: ctx.cat,
            is_dir: n.is_dir(),
            is_other: false,
            children,
        }
    }

    /// The biggest files in the scan, largest first. Uses a bounded heap, no full sort.
    pub fn largest_files(&self, limit: usize) -> Vec<FileEntry> {
        if limit == 0 {
            return Vec::new();
        }
        let mut heap: BinaryHeap<Reverse<(u64, NodeId)>> = BinaryHeap::with_capacity(limit + 1);
        for (i, n) in self.nodes.iter().enumerate() {
            if n.is_dir() || n.own_size == 0 || n.has(flags::HARDLINK_DUP) {
                continue;
            }
            if heap.len() < limit {
                heap.push(Reverse((n.own_size, i as NodeId)));
            } else if heap.peek().is_some_and(|Reverse((s, _))| n.own_size > *s) {
                heap.pop();
                heap.push(Reverse((n.own_size, i as NodeId)));
            }
        }
        let mut top: Vec<(u64, NodeId)> = heap.into_iter().map(|Reverse(x)| x).collect();
        top.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        top.into_iter()
            .map(|(size, id)| FileEntry {
                id,
                name: self.name(id).to_string(),
                path: self.path_of(id),
                size,
                modified: self.nodes[id as usize].modified(),
                category: self.effective_category(id),
            })
            .collect()
    }
}
