//! Compact node layout shared by both scanners.

use fazasanj_model::{Category, NodeFlags};

pub type NodeId = u32;

/// Marks "no node" in parent links.
pub const NONE: u32 = u32::MAX;
/// Marks "no time".
pub const NO_TIME: i64 = i64::MIN;
const UNTAGGED: u8 = 0xFF;
const NO_RULE: u32 = u32::MAX;

pub mod flags {
    pub const DIR: u16 = 1 << 0;
    pub const CLOUD_ONLY: u16 = 1 << 1;
    pub const ACCESS_DENIED: u16 = 1 << 2;
    pub const REPARSE: u16 = 1 << 3;
    pub const COMPRESSED: u16 = 1 << 4;
    pub const SPARSE: u16 = 1 << 5;
    pub const HARDLINK_DUP: u16 = 1 << 6;
    pub const SYSTEM: u16 = 1 << 7;
    pub const HIDDEN: u16 = 1 << 8;
}

/// One file or folder. 56 bytes.
///
/// In a finished tree nodes are stored in depth first preorder with children sorted by size,
/// so the subtree of `n` is exactly the id range `n..n.end`.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Node {
    /// Allocated bytes of this entry alone (0 for folders).
    pub own_size: u64,
    /// Own plus everything below.
    pub total_size: u64,
    /// Newest modification time in the subtree, unix ms, or `NO_TIME`.
    pub modified: i64,
    pub parent: u32,
    /// One past the last node of the subtree. During building it holds the subtree count.
    pub end: u32,
    pub name: u32,
    pub child_count: u32,
    pub file_count: u32,
    pub dir_count: u32,
    pub rule: u32,
    pub flags: u16,
    pub category: u8,
    _pad: u8,
}

const _: () = assert!(std::mem::size_of::<Node>() == 56);

impl Node {
    pub fn new(parent: u32, name: u32, own_size: u64, modified: i64, flags: u16) -> Self {
        Self {
            own_size,
            total_size: 0,
            modified,
            parent,
            end: 0,
            name,
            child_count: 0,
            file_count: 0,
            dir_count: 0,
            rule: NO_RULE,
            flags,
            category: UNTAGGED,
            _pad: 0,
        }
    }

    pub fn is_dir(&self) -> bool {
        self.flags & flags::DIR != 0
    }

    pub fn has(&self, flag: u16) -> bool {
        self.flags & flag != 0
    }

    pub fn modified(&self) -> Option<i64> {
        (self.modified != NO_TIME).then_some(self.modified)
    }

    pub fn tag(&self) -> Option<(Category, Option<u32>)> {
        let cat = category_from_u8(self.category)?;
        Some((cat, (self.rule != NO_RULE).then_some(self.rule)))
    }

    pub fn set_tag(&mut self, category: Category, rule: Option<u32>) {
        self.category = category_to_u8(category);
        self.rule = rule.unwrap_or(NO_RULE);
    }

    pub fn clear_tag(&mut self) {
        self.category = UNTAGGED;
        self.rule = NO_RULE;
    }

    pub fn model_flags(&self) -> NodeFlags {
        NodeFlags {
            cloud_only: self.has(flags::CLOUD_ONLY),
            access_denied: self.has(flags::ACCESS_DENIED),
            reparse: self.has(flags::REPARSE),
            compressed: self.has(flags::COMPRESSED),
            sparse: self.has(flags::SPARSE),
            hardlink_dup: self.has(flags::HARDLINK_DUP),
            system: self.has(flags::SYSTEM),
        }
    }
}

const CATEGORIES: [Category; 11] = [
    Category::System,
    Category::Apps,
    Category::Games,
    Category::Media,
    Category::Dev,
    Category::Cache,
    Category::UserFiles,
    Category::Virtualization,
    Category::Messaging,
    Category::Browsers,
    Category::Unknown,
];

/// Every category, in storage order.
pub fn all_categories() -> &'static [Category; 11] {
    &CATEGORIES
}

pub fn category_to_u8(c: Category) -> u8 {
    CATEGORIES.iter().position(|&x| x == c).map_or(UNTAGGED, |i| i as u8)
}

pub fn category_from_u8(v: u8) -> Option<Category> {
    CATEGORIES.get(v as usize).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_roundtrip() {
        let mut n = Node::new(NONE, 0, 0, NO_TIME, 0);
        assert_eq!(n.tag(), None);
        n.set_tag(Category::Dev, Some(7));
        assert_eq!(n.tag(), Some((Category::Dev, Some(7))));
        n.set_tag(Category::Unknown, None);
        assert_eq!(n.tag(), Some((Category::Unknown, None)));
        n.clear_tag();
        assert_eq!(n.tag(), None);
        for c in CATEGORIES {
            assert_eq!(category_from_u8(category_to_u8(c)), Some(c));
        }
    }
}
