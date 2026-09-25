//! String interner for node names. Names like `cache` or `index.js` repeat a lot, so each
//! distinct name is stored once in one big buffer.

use std::hash::BuildHasher;

use hashbrown::{DefaultHashBuilder, HashTable};

/// Read only name storage kept by the finished tree.
#[derive(Debug, Default)]
pub struct Names {
    buf: String,
    /// `start << 16 | len`. Names are at most 255 UTF-16 units, so 16 bits of length is plenty.
    spans: Vec<u64>,
}

impl Names {
    pub fn get(&self, id: u32) -> &str {
        let Some(&span) = self.spans.get(id as usize) else { return "" };
        let start = (span >> 16) as usize;
        let len = (span & 0xFFFF) as usize;
        self.buf.get(start..start + len).unwrap_or("")
    }

    pub fn len(&self) -> usize {
        self.spans.len()
    }

    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }

    pub fn heap_bytes(&self) -> usize {
        self.buf.capacity() + self.spans.capacity() * 8
    }
}

/// Build time interner. `finish` drops the lookup table and keeps only `Names`.
pub struct NameInterner {
    names: Names,
    table: HashTable<u32>,
    hasher: DefaultHashBuilder,
}

impl Default for NameInterner {
    fn default() -> Self {
        Self::new()
    }
}

impl NameInterner {
    pub fn new() -> Self {
        let mut s = Self { names: Names::default(), table: HashTable::new(), hasher: DefaultHashBuilder::default() };
        // Id 0 is the empty name, used for the root.
        s.intern("");
        s
    }

    pub fn intern(&mut self, name: &str) -> u32 {
        // Truncate at a char boundary if a name is somehow longer than the span allows.
        let mut end = name.len().min(0xFFFF);
        while !name.is_char_boundary(end) {
            end -= 1;
        }
        let name = &name[..end];

        let hash = self.hasher.hash_one(name);
        let names = &self.names;
        if let Some(&id) = self.table.find(hash, |&id| names.get(id) == name) {
            return id;
        }
        let id = self.names.spans.len() as u32;
        let start = self.names.buf.len() as u64;
        self.names.buf.push_str(name);
        self.names.spans.push((start << 16) | name.len() as u64);
        let (names, hasher) = (&self.names, &self.hasher);
        self.table.insert_unique(hash, id, |&i| hasher.hash_one(names.get(i)));
        id
    }

    pub fn get(&self, id: u32) -> &str {
        self.names.get(id)
    }

    pub fn finish(mut self) -> Names {
        self.names.buf.shrink_to_fit();
        self.names.spans.shrink_to_fit();
        self.names
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedupes() {
        let mut i = NameInterner::new();
        let a = i.intern("node_modules");
        let b = i.intern("index.js");
        let c = i.intern("node_modules");
        assert_eq!(a, c);
        assert_ne!(a, b);
        assert_eq!(i.intern(""), 0);
        let n = i.finish();
        assert_eq!(n.get(a), "node_modules");
        assert_eq!(n.get(b), "index.js");
        assert_eq!(n.get(999), "");
    }

    #[test]
    fn unicode_names() {
        let mut i = NameInterner::new();
        let a = i.intern("فایل‌ها");
        assert_eq!(i.get(a), "فایل‌ها");
    }
}
