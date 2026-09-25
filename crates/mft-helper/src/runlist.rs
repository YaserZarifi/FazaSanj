//! Data run (mapping pairs) decoding, used to find `$MFT` itself when
//! FSCTL_GET_RETRIEVAL_POINTERS is not available.

use fazasanj_platform::Extent;

use crate::record::{apply_fixups, RecordError, ATTR_DATA};

/// Decodes a mapping pairs array starting at VCN 0.
pub fn decode_runs(mut b: &[u8]) -> Result<Vec<Extent>, RecordError> {
    let mut out = Vec::new();
    let mut vcn = 0u64;
    let mut lcn = 0i64;
    while let Some(&header) = b.first() {
        if header == 0 {
            break;
        }
        let len_size = (header & 0x0F) as usize;
        let off_size = (header >> 4) as usize;
        if len_size == 0 || len_size > 8 || off_size > 8 || b.len() < 1 + len_size + off_size {
            return Err(RecordError::Truncated);
        }
        let mut len_bytes = [0u8; 8];
        len_bytes[..len_size].copy_from_slice(&b[1..1 + len_size]);
        let clusters = u64::from_le_bytes(len_bytes);
        let target = if off_size == 0 {
            None
        } else {
            let raw = &b[1 + len_size..1 + len_size + off_size];
            // Sign extend the relative offset.
            let fill = if raw[off_size - 1] & 0x80 != 0 { 0xFF } else { 0 };
            let mut off_bytes = [fill; 8];
            off_bytes[..off_size].copy_from_slice(raw);
            lcn += i64::from_le_bytes(off_bytes);
            Some(u64::try_from(lcn).map_err(|_| RecordError::Truncated)?)
        };
        out.push(Extent { vcn, lcn: target, clusters });
        vcn += clusters;
        b = &b[1 + len_size + off_size..];
    }
    Ok(out)
}

/// Extents of the unnamed `$DATA` stream in MFT record 0 (the `$MFT` file itself).
/// If `$MFT` is so fragmented that its runs continue in an extension record, only the first
/// part is returned; the caller should prefer FSCTL_GET_RETRIEVAL_POINTERS.
pub fn mft_extents_from_record(rec: &mut [u8]) -> Result<Vec<Extent>, RecordError> {
    apply_fixups(rec)?;
    let used = (u32::from_le_bytes([rec[0x18], rec[0x19], rec[0x1A], rec[0x1B]]) as usize).min(rec.len());
    let mut off = u16::from_le_bytes([rec[0x14], rec[0x15]]) as usize;
    while off + 0x40 <= used {
        let kind = u32::from_le_bytes([rec[off], rec[off + 1], rec[off + 2], rec[off + 3]]);
        let len = u32::from_le_bytes([rec[off + 4], rec[off + 5], rec[off + 6], rec[off + 7]]) as usize;
        if kind == 0xFFFF_FFFF || len == 0 || off + len > used {
            break;
        }
        let a = &rec[off..off + len];
        if kind == ATTR_DATA && a[8] != 0 && a[9] == 0 {
            let runs_off = u16::from_le_bytes([a[0x20], a[0x21]]) as usize;
            return decode_runs(a.get(runs_off..).ok_or(RecordError::Truncated)?);
        }
        off += len;
    }
    Err(RecordError::Truncated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_runs_with_negative_offset_and_hole() {
        // 0x20 clusters at LCN 0x1000, then a 0x10 cluster hole, then 8 clusters at 0x1000 - 0x100.
        let runs = [0x21, 0x20, 0x00, 0x10, 0x01, 0x10, 0x21, 0x08, 0x00, 0xFF, 0x00];
        let e = decode_runs(&runs).expect("runs");
        assert_eq!(
            e,
            vec![
                Extent { vcn: 0, lcn: Some(0x1000), clusters: 0x20 },
                Extent { vcn: 0x20, lcn: None, clusters: 0x10 },
                Extent { vcn: 0x30, lcn: Some(0x0F00), clusters: 8 },
            ]
        );
    }

    #[test]
    fn truncated_runs_fail() {
        assert!(decode_runs(&[0x21, 0x20]).is_err());
        assert!(decode_runs(&[]).expect("empty").is_empty());
    }
}
