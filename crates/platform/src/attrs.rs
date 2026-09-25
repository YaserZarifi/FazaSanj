//! File attribute and reparse tag helpers. Pure bit tests, no system calls.

pub const FILE_ATTRIBUTE_READONLY: u32 = 0x0000_0001;
pub const FILE_ATTRIBUTE_HIDDEN: u32 = 0x0000_0002;
pub const FILE_ATTRIBUTE_SYSTEM: u32 = 0x0000_0004;
pub const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x0000_0010;
pub const FILE_ATTRIBUTE_SPARSE_FILE: u32 = 0x0000_0200;
pub const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
pub const FILE_ATTRIBUTE_COMPRESSED: u32 = 0x0000_0800;
pub const FILE_ATTRIBUTE_OFFLINE: u32 = 0x0000_1000;
pub const FILE_ATTRIBUTE_RECALL_ON_OPEN: u32 = 0x0004_0000;
pub const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x0040_0000;

pub const IO_REPARSE_TAG_MOUNT_POINT: u32 = 0xA000_0003;
pub const IO_REPARSE_TAG_SYMLINK: u32 = 0xA000_000C;
pub const IO_REPARSE_TAG_DEDUP: u32 = 0x8000_0013;
pub const IO_REPARSE_TAG_WOF: u32 = 0x8000_0017;
pub const IO_REPARSE_TAG_CLOUD: u32 = 0x9000_001A;

/// A `FILE_ATTRIBUTE_*` bit set with readable accessors.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Attributes(pub u32);

impl Attributes {
    pub fn is_dir(self) -> bool {
        self.0 & FILE_ATTRIBUTE_DIRECTORY != 0
    }
    pub fn is_reparse(self) -> bool {
        self.0 & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    pub fn is_compressed(self) -> bool {
        self.0 & FILE_ATTRIBUTE_COMPRESSED != 0
    }
    pub fn is_sparse(self) -> bool {
        self.0 & FILE_ATTRIBUTE_SPARSE_FILE != 0
    }
    pub fn is_system(self) -> bool {
        self.0 & FILE_ATTRIBUTE_SYSTEM != 0
    }
    pub fn is_hidden(self) -> bool {
        self.0 & FILE_ATTRIBUTE_HIDDEN != 0
    }
    /// OneDrive and other cloud providers mark files that are not on disk this way.
    pub fn is_cloud_placeholder(self) -> bool {
        self.0 & (FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS | FILE_ATTRIBUTE_RECALL_ON_OPEN | FILE_ATTRIBUTE_OFFLINE)
            != 0
    }
}

/// Junctions, symlinks and mount points point somewhere else. Walking into them would count
/// data twice or loop, so scanners never follow these.
pub fn is_name_surrogate(tag: u32) -> bool {
    tag & 0x2000_0000 != 0
}

/// Any of the `IO_REPARSE_TAG_CLOUD_*` variants (the low nibble of the third byte varies).
pub fn is_cloud_tag(tag: u32) -> bool {
    tag & 0xFFFF_0FFF == IO_REPARSE_TAG_CLOUD
}

/// Tags whose real on-disk size lives outside the unnamed data stream
/// (Windows compact OS compression and data dedup).
pub fn tag_hides_real_size(tag: u32) -> bool {
    tag == IO_REPARSE_TAG_WOF || tag == IO_REPARSE_TAG_DEDUP
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surrogates() {
        assert!(is_name_surrogate(IO_REPARSE_TAG_MOUNT_POINT));
        assert!(is_name_surrogate(IO_REPARSE_TAG_SYMLINK));
        assert!(!is_name_surrogate(IO_REPARSE_TAG_WOF));
        assert!(!is_name_surrogate(IO_REPARSE_TAG_CLOUD));
    }

    #[test]
    fn cloud_tags() {
        assert!(is_cloud_tag(0x9000_001A));
        assert!(is_cloud_tag(0x9000_F01A));
        assert!(!is_cloud_tag(IO_REPARSE_TAG_SYMLINK));
    }

    #[test]
    fn placeholder_bits() {
        assert!(Attributes(FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS).is_cloud_placeholder());
        assert!(Attributes(FILE_ATTRIBUTE_OFFLINE).is_cloud_placeholder());
        assert!(!Attributes(FILE_ATTRIBUTE_COMPRESSED).is_cloud_placeholder());
    }
}
