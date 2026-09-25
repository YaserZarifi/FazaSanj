//! How much space one entry really uses. Pure logic, no disk access.
//!
//! Rules, shared by both scanners:
//! - Allocated size, not logical size.
//! - Cloud placeholders (OneDrive and friends, not downloaded) count as 0 and get `CLOUD_ONLY`.
//! - Compressed, sparse and WOF / dedup files use the real on-disk size when we have it.
//! - Hard links count once. The copy that keeps the bytes is picked by a stable rank, the
//!   other copies get `HARDLINK_DUP` and size 0.
//! - Reparse points are flagged. Whether to walk into them is the walker's call.

use fazasanj_platform::{tag_hides_real_size, Attributes};

use crate::node::flags;

/// Where the size of a file should come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeSource {
    /// The allocation size from the directory entry or the MFT is right.
    Allocation,
    /// Not on disk, counts as zero.
    CloudOnly,
    /// Compressed, sparse or WOF: the directory entry can be wrong, ask for the real size.
    Precise,
}

pub fn size_source(attributes: u32, reparse_tag: u32) -> SizeSource {
    let a = Attributes(attributes);
    if a.is_cloud_placeholder() {
        SizeSource::CloudOnly
    } else if a.is_compressed() || a.is_sparse() || (a.is_reparse() && tag_hides_real_size(reparse_tag)) {
        SizeSource::Precise
    } else {
        SizeSource::Allocation
    }
}

/// Node flags that follow directly from the attributes.
pub fn attribute_flags(attributes: u32) -> u16 {
    let a = Attributes(attributes);
    let mut f = 0;
    if a.is_dir() {
        f |= flags::DIR;
    }
    if a.is_reparse() {
        f |= flags::REPARSE;
    }
    if a.is_compressed() {
        f |= flags::COMPRESSED;
    }
    if a.is_sparse() {
        f |= flags::SPARSE;
    }
    if a.is_system() {
        f |= flags::SYSTEM;
    }
    if a.is_hidden() {
        f |= flags::HIDDEN;
    }
    if a.is_cloud_placeholder() {
        f |= flags::CLOUD_ONLY;
    }
    f
}

/// Size and flags for one file. `precise` is the on-disk size from a separate query, when
/// `size_source` asked for one and it worked.
pub fn file_size(attributes: u32, reparse_tag: u32, allocation: u64, precise: Option<u64>) -> (u64, u16) {
    let f = attribute_flags(attributes) & !flags::DIR;
    let size = match size_source(attributes, reparse_tag) {
        SizeSource::CloudOnly => 0,
        SizeSource::Precise => precise.unwrap_or(allocation),
        SizeSource::Allocation => allocation,
    };
    (size, f)
}

/// Finds duplicate hard links.
///
/// `cands` holds `(file id, node)` for every file that could be a link. Within each group of
/// equal ids the node with the lowest `rank` keeps its size, the rest are returned.
pub fn hardlink_dups<K: Ord + Copy, R: Ord>(cands: &mut [(K, u32)], mut rank: impl FnMut(u32) -> R) -> Vec<u32> {
    cands.sort_unstable();
    let mut dups = Vec::new();
    let mut i = 0;
    while i < cands.len() {
        let mut j = i + 1;
        while j < cands.len() && cands[j].0 == cands[i].0 {
            j += 1;
        }
        if j - i > 1 {
            let group = &cands[i..j];
            let keep = group.iter().map(|&(_, n)| (rank(n), n)).min().map(|(_, n)| n);
            // The same node can show up twice in a group, the filter never marks the kept one.
            dups.extend(group.iter().map(|&(_, n)| n).filter(|&n| Some(n) != keep));
        }
        i = j;
    }
    dups.sort_unstable();
    dups.dedup();
    dups
}

#[cfg(test)]
mod tests {
    use super::*;
    use fazasanj_platform::{
        FILE_ATTRIBUTE_COMPRESSED, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_OFFLINE, FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS,
        FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_SPARSE_FILE, IO_REPARSE_TAG_CLOUD, IO_REPARSE_TAG_SYMLINK,
        IO_REPARSE_TAG_WOF,
    };

    #[test]
    fn normal_file_uses_allocation() {
        assert_eq!(file_size(0x20, 0, 8192, None), (8192, 0));
    }

    #[test]
    fn placeholders_count_zero() {
        let a = FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS | FILE_ATTRIBUTE_REPARSE_POINT;
        let (size, f) = file_size(a, IO_REPARSE_TAG_CLOUD, 1 << 30, None);
        assert_eq!(size, 0);
        assert!(f & flags::CLOUD_ONLY != 0);
        assert_eq!(file_size(FILE_ATTRIBUTE_OFFLINE, 0, 4096, None).0, 0);
    }

    #[test]
    fn hydrated_cloud_file_counts() {
        let (size, f) = file_size(FILE_ATTRIBUTE_REPARSE_POINT, IO_REPARSE_TAG_CLOUD, 4096, None);
        assert_eq!(size, 4096);
        assert_eq!(f & flags::CLOUD_ONLY, 0);
    }

    #[test]
    fn compressed_and_sparse_use_precise() {
        assert_eq!(size_source(FILE_ATTRIBUTE_COMPRESSED, 0), SizeSource::Precise);
        assert_eq!(size_source(FILE_ATTRIBUTE_SPARSE_FILE, 0), SizeSource::Precise);
        assert_eq!(size_source(FILE_ATTRIBUTE_REPARSE_POINT, IO_REPARSE_TAG_WOF), SizeSource::Precise);
        assert_eq!(size_source(FILE_ATTRIBUTE_REPARSE_POINT, IO_REPARSE_TAG_SYMLINK), SizeSource::Allocation);
        let (size, f) = file_size(FILE_ATTRIBUTE_COMPRESSED, 0, 1 << 20, Some(300_000));
        assert_eq!(size, 300_000);
        assert!(f & flags::COMPRESSED != 0);
        // Query failed: fall back to what the entry said.
        assert_eq!(file_size(FILE_ATTRIBUTE_SPARSE_FILE, 0, 4096, None).0, 4096);
    }

    #[test]
    fn dir_flag_only_from_attributes() {
        assert!(attribute_flags(FILE_ATTRIBUTE_DIRECTORY) & flags::DIR != 0);
        assert_eq!(file_size(FILE_ATTRIBUTE_DIRECTORY, 0, 0, None).1 & flags::DIR, 0);
    }

    #[test]
    fn hardlinks_counted_once_with_rank() {
        // Nodes 1 and 5 are the same file, 2 and 3 and 4 are another, 9 is alone.
        let mut c: Vec<(u64, u32)> = vec![(10, 5), (20, 3), (10, 1), (20, 2), (30, 9), (20, 4)];
        // Lower node id ranks first, except node 2 which ranks last (think: it lives in WinSxS).
        let dups = hardlink_dups(&mut c, |n| (n == 2, n));
        assert_eq!(dups, vec![2, 4, 5]);
    }

    #[test]
    fn same_node_twice_is_not_a_dup() {
        let mut c: Vec<(u128, u32)> = vec![(7, 3), (7, 3)];
        assert!(hardlink_dups(&mut c, |n| n).is_empty());
    }
}
