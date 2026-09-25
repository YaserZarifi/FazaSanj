//! NTFS FILE record parsing. Pure byte work, unit tested with synthetic records.
//!
//! Layout reference: the FILE record header, then attributes until the 0xFFFFFFFF end marker.
//! We read only what a disk usage tool needs: `$STANDARD_INFORMATION` (times, attributes),
//! `$FILE_NAME` (names and parents), `$DATA` (allocated size) and whether an
//! `$ATTRIBUTE_LIST` exists (then more attributes live in extension records).

pub const ATTR_STANDARD_INFORMATION: u32 = 0x10;
pub const ATTR_ATTRIBUTE_LIST: u32 = 0x20;
pub const ATTR_FILE_NAME: u32 = 0x30;
pub const ATTR_DATA: u32 = 0x80;
const ATTR_END: u32 = 0xFFFF_FFFF;

const RECORD_IN_USE: u16 = 0x0001;
const RECORD_IS_DIR: u16 = 0x0002;
/// NTFS applies the update sequence every 512 bytes, whatever the sector size.
const FIXUP_STRIDE: usize = 512;
const NAMESPACE_DOS: u8 = 2;
const REF_MASK: u64 = 0x0000_FFFF_FFFF_FFFF;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordError {
    /// Not a FILE record (never used, or zeroed).
    NotAFileRecord,
    /// Torn write or corruption: the update sequence does not match.
    BadFixup,
    Truncated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileName {
    /// Parent folder record number.
    pub parent: u64,
    pub name: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParsedRecord {
    pub in_use: bool,
    pub is_dir: bool,
    /// Base record number for extension records, 0 for base records.
    pub base_record: u64,
    pub has_attr_list: bool,
    /// FILETIME of the last content change.
    pub modified: Option<i64>,
    /// `FILE_ATTRIBUTE_*` from `$STANDARD_INFORMATION`.
    pub attributes: Option<u32>,
    /// Hard links (DOS 8.3 aliases are left out).
    pub names: Vec<FileName>,
    /// Allocated bytes of all data streams whose first extent is in this record.
    pub data_size: u64,
}

fn u16_at(b: &[u8], o: usize) -> Result<u16, RecordError> {
    b.get(o..o + 2).map(|s| u16::from_le_bytes([s[0], s[1]])).ok_or(RecordError::Truncated)
}

fn u32_at(b: &[u8], o: usize) -> Result<u32, RecordError> {
    b.get(o..o + 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]])).ok_or(RecordError::Truncated)
}

fn u64_at(b: &[u8], o: usize) -> Result<u64, RecordError> {
    let s = b.get(o..o + 8).ok_or(RecordError::Truncated)?;
    let mut a = [0u8; 8];
    a.copy_from_slice(s);
    Ok(u64::from_le_bytes(a))
}

/// Checks and undoes the update sequence array in place.
pub fn apply_fixups(rec: &mut [u8]) -> Result<(), RecordError> {
    let usa_off = u16_at(rec, 4)? as usize;
    let usa_count = u16_at(rec, 6)? as usize;
    if usa_count < 2 || (usa_count - 1) * FIXUP_STRIDE > rec.len() || usa_off + usa_count * 2 > rec.len() {
        return Err(RecordError::BadFixup);
    }
    let usn = [rec[usa_off], rec[usa_off + 1]];
    for i in 1..usa_count {
        let end = i * FIXUP_STRIDE - 2;
        if rec[end..end + 2] != usn {
            return Err(RecordError::BadFixup);
        }
        let (a, b) = (rec[usa_off + i * 2], rec[usa_off + i * 2 + 1]);
        rec[end] = a;
        rec[end + 1] = b;
    }
    Ok(())
}

/// Size of one `$DATA` attribute on disk, or `None` when this is a later extent
/// (its size fields are only valid in the extent that starts at VCN 0).
fn data_size(attr: &[u8]) -> Result<Option<u64>, RecordError> {
    let non_resident = attr.get(8).copied().ok_or(RecordError::Truncated)? != 0;
    let name_len = attr[9];
    if !non_resident {
        // Resident data sits inside the MFT record. Named resident streams (Zone.Identifier)
        // are ignored, the unnamed one reports its length rounded to 8 like directory listings do.
        if name_len != 0 {
            return Ok(Some(0));
        }
        let len = u32_at(attr, 0x10)? as u64;
        return Ok(Some((len + 7) & !7));
    }
    if u64_at(attr, 0x10)? != 0 {
        return Ok(None);
    }
    let flags = u16_at(attr, 0x0C)?;
    let compressed_or_sparse = flags & 0x00FF != 0 || flags & 0x8000 != 0;
    if compressed_or_sparse && attr.len() >= 0x48 {
        // "Total allocated": clusters really in use for compressed or sparse streams.
        return Ok(Some(u64_at(attr, 0x40)?));
    }
    Ok(Some(u64_at(attr, 0x28)?))
}

fn parse_file_name(v: &[u8]) -> Result<Option<FileName>, RecordError> {
    let parent = u64_at(v, 0)? & REF_MASK;
    let len = *v.get(0x40).ok_or(RecordError::Truncated)? as usize;
    let namespace = *v.get(0x41).ok_or(RecordError::Truncated)?;
    if namespace == NAMESPACE_DOS {
        return Ok(None);
    }
    let raw = v.get(0x42..0x42 + len * 2).ok_or(RecordError::Truncated)?;
    let units = raw.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c));
    let name = char::decode_utf16(units).map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER)).collect();
    Ok(Some(FileName { parent, name }))
}

fn resident_value(attr: &[u8]) -> Result<&[u8], RecordError> {
    let len = u32_at(attr, 0x10)? as usize;
    let off = u16_at(attr, 0x14)? as usize;
    attr.get(off..off + len).ok_or(RecordError::Truncated)
}

/// Parses one FILE record. Applies the fixups first, so `rec` is modified.
pub fn parse_record(rec: &mut [u8]) -> Result<ParsedRecord, RecordError> {
    if rec.len() < 0x30 || &rec[0..4] != b"FILE" {
        return Err(RecordError::NotAFileRecord);
    }
    apply_fixups(rec)?;
    let flags = u16_at(rec, 0x16)?;
    let mut out = ParsedRecord {
        in_use: flags & RECORD_IN_USE != 0,
        is_dir: flags & RECORD_IS_DIR != 0,
        base_record: u64_at(rec, 0x20)? & REF_MASK,
        ..ParsedRecord::default()
    };
    if !out.in_use {
        return Ok(out);
    }
    let used = (u32_at(rec, 0x18)? as usize).min(rec.len());
    let mut off = u16_at(rec, 0x14)? as usize;
    while off + 8 <= used {
        let kind = u32_at(rec, off)?;
        if kind == ATTR_END {
            break;
        }
        let len = u32_at(rec, off + 4)? as usize;
        if len < 0x18 || off + len > used {
            return Err(RecordError::Truncated);
        }
        let attr = &rec[off..off + len];
        let resident = attr[8] == 0;
        match kind {
            ATTR_STANDARD_INFORMATION if resident => {
                let v = resident_value(attr)?;
                out.modified = Some(u64_at(v, 0x08)? as i64);
                out.attributes = Some(u32_at(v, 0x20)?);
            }
            ATTR_ATTRIBUTE_LIST => out.has_attr_list = true,
            ATTR_FILE_NAME if resident => {
                if let Some(n) = parse_file_name(resident_value(attr)?)? {
                    out.names.push(n);
                }
            }
            ATTR_DATA => {
                if let Some(s) = data_size(attr)? {
                    out.data_size += s;
                }
            }
            _ => {}
        }
        off += len;
    }
    Ok(out)
}

#[cfg(test)]
pub(crate) mod test_util {
    //! Builds synthetic FILE records for tests.

    use super::*;

    pub struct RecordBuilder {
        buf: Vec<u8>,
        off: usize,
    }

    impl RecordBuilder {
        pub fn new(size: usize, flags: u16, base: u64) -> Self {
            let mut buf = vec![0u8; size];
            buf[0..4].copy_from_slice(b"FILE");
            let usa_count = (size / FIXUP_STRIDE + 1) as u16;
            buf[4..6].copy_from_slice(&0x30u16.to_le_bytes());
            buf[6..8].copy_from_slice(&usa_count.to_le_bytes());
            buf[0x14..0x16].copy_from_slice(&0x38u16.to_le_bytes());
            buf[0x16..0x18].copy_from_slice(&flags.to_le_bytes());
            buf[0x1C..0x20].copy_from_slice(&(size as u32).to_le_bytes());
            buf[0x20..0x28].copy_from_slice(&base.to_le_bytes());
            Self { buf, off: 0x38 }
        }

        fn attr(&mut self, kind: u32, body: &[u8]) {
            let len = body.len().div_ceil(8) * 8;
            let o = self.off;
            self.buf[o..o + body.len()].copy_from_slice(body);
            self.buf[o..o + 4].copy_from_slice(&kind.to_le_bytes());
            self.buf[o + 4..o + 8].copy_from_slice(&(len as u32).to_le_bytes());
            self.off += len;
        }

        pub fn resident(&mut self, kind: u32, name_len: u8, value: &[u8]) -> &mut Self {
            let mut a = vec![0u8; 0x18 + value.len()];
            a[9] = name_len;
            a[0x10..0x14].copy_from_slice(&(value.len() as u32).to_le_bytes());
            a[0x14..0x16].copy_from_slice(&0x18u16.to_le_bytes());
            a[0x18..].copy_from_slice(value);
            self.attr(kind, &a);
            self
        }

        pub fn std_info(&mut self, modified: i64, attrs: u32) -> &mut Self {
            let mut v = vec![0u8; 0x48];
            v[8..16].copy_from_slice(&modified.to_le_bytes());
            v[0x20..0x24].copy_from_slice(&attrs.to_le_bytes());
            self.resident(ATTR_STANDARD_INFORMATION, 0, &v)
        }

        pub fn file_name(&mut self, parent: u64, name: &str, namespace: u8) -> &mut Self {
            let units: Vec<u16> = name.encode_utf16().collect();
            let mut v = vec![0u8; 0x42 + units.len() * 2];
            // Sequence number in the high 16 bits must be masked off.
            v[0..8].copy_from_slice(&(parent | (3u64 << 48)).to_le_bytes());
            v[0x40] = units.len() as u8;
            v[0x41] = namespace;
            for (i, u) in units.iter().enumerate() {
                v[0x42 + i * 2..0x44 + i * 2].copy_from_slice(&u.to_le_bytes());
            }
            self.resident(ATTR_FILE_NAME, 0, &v)
        }

        pub fn nonresident_data(&mut self, lowest_vcn: u64, alloc: u64, total: Option<u64>, flags: u16) -> &mut Self {
            let mut a = vec![0u8; if total.is_some() { 0x48 } else { 0x40 }];
            a[8] = 1;
            a[0x0C..0x0E].copy_from_slice(&flags.to_le_bytes());
            a[0x10..0x18].copy_from_slice(&lowest_vcn.to_le_bytes());
            a[0x28..0x30].copy_from_slice(&alloc.to_le_bytes());
            if let Some(t) = total {
                a[0x40..0x48].copy_from_slice(&t.to_le_bytes());
            }
            self.attr(ATTR_DATA, &a);
            self
        }

        /// Ends the attributes and writes the update sequence, like NTFS does on disk.
        pub fn build(&mut self) -> Vec<u8> {
            let o = self.off;
            self.buf[o..o + 4].copy_from_slice(&ATTR_END.to_le_bytes());
            self.buf[0x18..0x1C].copy_from_slice(&((o + 8) as u32).to_le_bytes());
            let mut b = self.buf.clone();
            let usn = [0x34u8, 0x12];
            b[0x30..0x32].copy_from_slice(&usn);
            for i in 1..=(b.len() / FIXUP_STRIDE) {
                let end = i * FIXUP_STRIDE - 2;
                b[0x30 + i * 2] = b[end];
                b[0x30 + i * 2 + 1] = b[end + 1];
                b[end..end + 2].copy_from_slice(&usn);
            }
            b
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_util::RecordBuilder;
    use super::*;

    #[test]
    fn fixups_restore_sector_tails() {
        let mut rb = RecordBuilder::new(1024, RECORD_IN_USE, 0);
        // Put a long name across the first sector boundary so the fixup matters.
        let long = "x".repeat(200);
        rb.std_info(1, 0x20).file_name(5, &long, 1);
        let mut rec = rb.build();
        let p = parse_record(&mut rec).expect("parse");
        assert_eq!(p.names[0].name, long);
    }

    #[test]
    fn torn_record_is_rejected() {
        let mut rec = RecordBuilder::new(1024, RECORD_IN_USE, 0).std_info(1, 0).build();
        rec[1022] ^= 0xFF;
        assert_eq!(parse_record(&mut rec), Err(RecordError::BadFixup));
        let mut zero = vec![0u8; 1024];
        assert_eq!(parse_record(&mut zero), Err(RecordError::NotAFileRecord));
    }

    #[test]
    fn resident_data_and_names() {
        let mut rec = RecordBuilder::new(1024, RECORD_IN_USE, 0)
            .std_info(132_000_000_000_000_000, 0x20)
            .file_name(40, "LONGFI~1.TXT", NAMESPACE_DOS)
            .file_name(40, "long file name.txt", 1)
            .resident(ATTR_DATA, 0, &[1u8; 100])
            .resident(ATTR_DATA, 15, &[2u8; 26])
            .build();
        let p = parse_record(&mut rec).expect("parse");
        assert!(p.in_use && !p.is_dir);
        assert_eq!(p.names, vec![FileName { parent: 40, name: "long file name.txt".into() }]);
        assert_eq!(p.data_size, 104);
        assert_eq!(p.attributes, Some(0x20));
        assert_eq!(p.modified, Some(132_000_000_000_000_000));
    }

    #[test]
    fn nonresident_plain_compressed_and_later_extents() {
        let mut rec = RecordBuilder::new(1024, RECORD_IN_USE, 0)
            .std_info(1, 0x800)
            .file_name(5, "big.vhdx", 3)
            .nonresident_data(0, 1 << 30, None, 0)
            // Compressed named stream: the total allocated field wins.
            .nonresident_data(0, 1 << 20, Some(4096 * 3), 0x0001)
            // A later extent of some stream: sizes are not valid here.
            .nonresident_data(500, 999_999, None, 0)
            .build();
        let p = parse_record(&mut rec).expect("parse");
        assert_eq!(p.data_size, (1 << 30) + 4096 * 3);
    }

    #[test]
    fn hard_links_and_dirs() {
        let mut rec = RecordBuilder::new(1024, RECORD_IN_USE | RECORD_IS_DIR, 0)
            .std_info(1, 0x10)
            .file_name(5, "a", 0)
            .build();
        let p = parse_record(&mut rec).expect("parse");
        assert!(p.is_dir);

        let mut rec = RecordBuilder::new(1024, RECORD_IN_USE, 0)
            .std_info(1, 0x20)
            .file_name(40, "kernel.dll", 3)
            .file_name(77, "kernel.dll", 0)
            .build();
        let p = parse_record(&mut rec).expect("parse");
        assert_eq!(p.names.iter().map(|n| n.parent).collect::<Vec<_>>(), vec![40, 77]);
    }

    #[test]
    fn extension_record_and_attribute_list() {
        let mut base = RecordBuilder::new(1024, RECORD_IN_USE, 0)
            .std_info(1, 0x20)
            .resident(ATTR_ATTRIBUTE_LIST, 0, &[0u8; 64])
            .file_name(5, "huge.iso", 1)
            .build();
        let p = parse_record(&mut base).expect("base");
        assert!(p.has_attr_list);
        assert_eq!(p.data_size, 0);

        let mut ext = RecordBuilder::new(1024, RECORD_IN_USE, 1234).nonresident_data(0, 5 << 30, None, 0).build();
        let e = parse_record(&mut ext).expect("ext");
        assert_eq!(e.base_record, 1234);
        assert_eq!(e.data_size, 5 << 30);
    }

    #[test]
    fn unused_record() {
        let mut rec = RecordBuilder::new(1024, 0, 0).std_info(1, 0).build();
        let p = parse_record(&mut rec).expect("parse");
        assert!(!p.in_use);
        assert!(p.names.is_empty());
    }
}
