//! Turns parsed records into wire records.
//!
//! Most files fit in one FILE record and are sent right away. Files with an `$ATTRIBUTE_LIST`
//! keep some attributes (often the `$DATA` of big fragmented files, sometimes extra names) in
//! extension records. Those are collected here by base record number and sent at the end.

use std::collections::HashMap;

use fazasanj_platform::filetime_to_unix_ms;

use crate::record::{FileName, ParsedRecord};
use crate::wire::{WireRecord, FLAG_DIR, ROOT_RECORD};

/// Records below this are NTFS metadata files ($MFT, $LogFile, $Bitmap...), hidden from normal
/// directory listings too, so both scanners leave them out. The root folder (5) is the exception.
const FIRST_USER_RECORD: u64 = 16;
/// `$Extend` holds more metadata ($UsnJrnl, $Quota...).
const EXTEND_RECORD: u64 = 11;

#[derive(Debug, Default)]
struct Partial {
    base_seen: bool,
    is_dir: bool,
    modified: Option<i64>,
    attributes: Option<u32>,
    names: Vec<FileName>,
    data_size: u64,
}

#[derive(Debug, Default)]
pub struct Merger {
    partial: HashMap<u64, Partial>,
}

fn emit_all(
    rec: u64,
    is_dir: bool,
    modified: Option<i64>,
    attributes: u32,
    names: &[FileName],
    size: u64,
    emit: &mut impl FnMut(&WireRecord),
) {
    if rec < FIRST_USER_RECORD && rec != ROOT_RECORD {
        return;
    }
    let modified = modified.and_then(filetime_to_unix_ms).unwrap_or(i64::MIN);
    for n in names {
        if n.parent == EXTEND_RECORD && rec != ROOT_RECORD {
            continue;
        }
        emit(&WireRecord {
            rec,
            parent: n.parent,
            size: if is_dir { 0 } else { size },
            modified,
            attributes,
            flags: if is_dir { FLAG_DIR } else { 0 },
            name: &n.name,
        });
    }
}

impl Merger {
    pub fn add(&mut self, rec: u64, p: ParsedRecord, emit: &mut impl FnMut(&WireRecord)) {
        if !p.in_use {
            return;
        }
        if p.base_record != 0 {
            let part = self.partial.entry(p.base_record).or_default();
            part.names.extend(p.names);
            part.data_size += p.data_size;
            if p.modified.is_some() {
                part.modified = p.modified;
                part.attributes = p.attributes;
            }
            return;
        }
        if p.has_attr_list {
            let part = self.partial.entry(rec).or_default();
            part.base_seen = true;
            part.is_dir = p.is_dir;
            part.names.extend(p.names);
            part.data_size += p.data_size;
            if p.modified.is_some() {
                part.modified = p.modified;
                part.attributes = p.attributes;
            }
            return;
        }
        emit_all(rec, p.is_dir, p.modified, p.attributes.unwrap_or(0), &p.names, p.data_size, emit);
    }

    /// Sends files that were spread over several records. Extension records whose base record
    /// was not in use are stale and dropped.
    pub fn finish(self, emit: &mut impl FnMut(&WireRecord)) {
        let mut recs: Vec<(u64, Partial)> = self.partial.into_iter().filter(|(_, p)| p.base_seen).collect();
        recs.sort_by_key(|(r, _)| *r);
        for (rec, p) in recs {
            emit_all(rec, p.is_dir, p.modified, p.attributes.unwrap_or(0), &p.names, p.data_size, emit);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base(names: &[(u64, &str)], size: u64, list: bool) -> ParsedRecord {
        ParsedRecord {
            in_use: true,
            is_dir: false,
            base_record: 0,
            has_attr_list: list,
            modified: Some(116_444_736_000_000_000 + 10_000 * 42),
            attributes: Some(0x20),
            names: names.iter().map(|&(parent, n)| FileName { parent, name: n.into() }).collect(),
            data_size: size,
        }
    }

    fn collect(f: impl FnOnce(&mut dyn FnMut(&WireRecord))) -> Vec<(u64, u64, String, u64, i64)> {
        let mut out = Vec::new();
        f(&mut |r: &WireRecord| out.push((r.rec, r.parent, r.name.to_string(), r.size, r.modified)));
        out
    }

    #[test]
    fn simple_records_go_out_right_away() {
        let mut m = Merger::default();
        let out = collect(|e| {
            let mut e = e;
            m.add(100, base(&[(5, "a.txt")], 4096, false), &mut e);
            m.add(101, base(&[(40, "x.dll"), (41, "x.dll")], 8192, false), &mut e);
        });
        assert_eq!(out.len(), 3);
        assert_eq!(out[0], (100, 5, "a.txt".into(), 4096, 42));
        assert_eq!(out[2].0, 101);
        assert_eq!(out[2].3, 8192, "every link carries the size, the app dedupes");
    }

    #[test]
    fn attribute_list_files_are_merged() {
        let mut m = Merger::default();
        let ext = ParsedRecord { in_use: true, base_record: 200, data_size: 5 << 30, ..ParsedRecord::default() };
        let ext_name = ParsedRecord {
            in_use: true,
            base_record: 200,
            names: vec![FileName { parent: 77, name: "second-link.iso".into() }],
            ..ParsedRecord::default()
        };
        let stale = ParsedRecord { in_use: true, base_record: 999, data_size: 1, ..ParsedRecord::default() };
        let out = std::cell::RefCell::new(Vec::new());
        let mut e = |r: &WireRecord| out.borrow_mut().push((r.rec, r.parent, r.name.to_string(), r.size));
        // Extension record can come before its base.
        m.add(300, ext, &mut e);
        m.add(200, base(&[(5, "huge.iso")], 0, true), &mut e);
        m.add(301, ext_name, &mut e);
        m.add(302, stale, &mut e);
        assert!(out.borrow().is_empty());
        m.finish(&mut e);
        assert_eq!(
            out.into_inner(),
            vec![(200, 5, "huge.iso".into(), 5 << 30), (200, 77, "second-link.iso".into(), 5 << 30)]
        );
    }

    #[test]
    fn metadata_records_are_skipped() {
        let mut m = Merger::default();
        let out = collect(|e| {
            let mut e = e;
            m.add(0, base(&[(5, "$MFT")], 1 << 30, false), &mut e);
            m.add(5, ParsedRecord { is_dir: true, ..base(&[(5, ".")], 0, false) }, &mut e);
            m.add(40, base(&[(EXTEND_RECORD, "$UsnJrnl")], 1 << 30, false), &mut e);
        });
        assert_eq!(out.len(), 1);
        assert_eq!((out[0].0, out[0].2.as_str(), out[0].3), (5, ".", 0));
    }
}
