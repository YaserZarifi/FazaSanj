//! File type groups by extension.

use std::collections::HashMap;

use fazasanj_model::{ExtensionStat, TypeGroup, TypeGroupKind};

use crate::node::NodeId;
use crate::tree::ScanTree;

const TOP_EXTENSIONS: usize = 8;

/// Lowercase extension of a file name, without the dot. Names like `.gitignore` have none.
pub fn extension_of(name: &str, buf: &mut String) {
    buf.clear();
    let Some((stem, ext)) = name.rsplit_once('.') else { return };
    if stem.is_empty() || ext.is_empty() || ext.len() > 16 || ext.contains(' ') {
        return;
    }
    if ext.is_ascii() {
        buf.extend(ext.chars().map(|c| c.to_ascii_lowercase()));
    } else {
        buf.push_str(&ext.to_lowercase());
    }
}

/// Group for an extension key. Special system files use their full lowercase name as key.
pub fn group_of(ext: &str) -> TypeGroupKind {
    use TypeGroupKind::*;
    match ext {
        "pagefile.sys" | "hiberfil.sys" | "swapfile.sys" | "memory.dmp" => System,
        "mp4" | "mkv" | "avi" | "mov" | "wmv" | "webm" | "m4v" | "flv" | "mpg" | "mpeg" | "ts" | "m2ts" | "3gp" => Video,
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tif" | "tiff" | "heic" | "heif" | "webp" | "raw" | "cr2" | "cr3"
        | "nef" | "arw" | "dng" | "psd" | "svg" | "ico" => Images,
        "mp3" | "flac" | "wav" | "m4a" | "aac" | "ogg" | "opus" | "wma" | "aiff" => Audio,
        "zip" | "rar" | "7z" | "tar" | "gz" | "tgz" | "bz2" | "xz" | "zst" | "cab" | "lz4" => Archives,
        "exe" | "msi" | "msix" | "msixbundle" | "appx" | "appxbundle" | "msu" | "msp" => Installers,
        "iso" | "img" | "vhd" | "vhdx" | "vmdk" | "vdi" | "qcow2" | "avhdx" | "wim" | "esd" => DiskImages,
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "ods" | "odp" | "txt" | "rtf" | "md"
        | "epub" | "csv" | "one" => Documents,
        "rs" | "c" | "h" | "cpp" | "hpp" | "cs" | "java" | "kt" | "py" | "js" | "mjs" | "cjs" | "tsx"
        | "jsx" | "go" | "rb" | "php" | "swift" | "json" | "xml" | "yml" | "yaml" | "toml" | "html" | "css"
        | "scss" | "vue" | "svelte" | "sql" | "sh" | "ps1" | "bat" | "class" | "jar" | "pyc" | "pdb" | "obj"
        | "o" | "lib" | "a" | "rlib" | "rmeta" | "node" | "map" => Code,
        "dll" | "sys" | "ocx" | "drv" | "efi" | "mui" | "cpl" | "scr" => Executables,
        "log" | "etl" | "evtx" | "dmp" | "mum" | "cat" | "manifest" | "pf" | "blf" | "regtrans-ms" | "edb"
        | "chk" | "jrs" => System,
        _ => Other,
    }
}

fn is_system_file(name: &str) -> bool {
    ["pagefile.sys", "hiberfil.sys", "swapfile.sys", "memory.dmp"].iter().any(|s| name.eq_ignore_ascii_case(s))
}

#[derive(Default)]
struct ExtAgg {
    map: HashMap<String, (u64, u64)>,
    buf: String,
}

impl ExtAgg {
    fn add(&mut self, name: &str, size: u64) {
        if is_system_file(name) {
            self.buf = name.to_ascii_lowercase();
        } else {
            extension_of(name, &mut self.buf);
        }
        let key = self.buf.as_str();
        if let Some(v) = self.map.get_mut(key) {
            v.0 += size;
            v.1 += 1;
        } else {
            self.map.insert(key.to_string(), (size, 1));
        }
    }

    fn into_sorted(self) -> Vec<ExtensionStat> {
        let mut v: Vec<ExtensionStat> =
            self.map.into_iter().map(|(ext, (bytes, files))| ExtensionStat { ext, bytes, files }).collect();
        v.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.ext.cmp(&b.ext)));
        v
    }
}

impl ScanTree {
    fn file_ids_in(&self, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        let end = self.node(id).map_or(id, |n| n.end);
        (id..end).filter(move |&i| !self.nodes[i as usize].is_dir())
    }

    /// Space per extension inside a subtree, largest first, at most `top_n` entries.
    pub fn subtree_extension_stats(&self, id: NodeId, top_n: usize) -> Vec<ExtensionStat> {
        let mut agg = ExtAgg::default();
        for f in self.file_ids_in(id) {
            let n = &self.nodes[f as usize];
            agg.add(self.names.get(n.name), n.own_size);
        }
        let mut v = agg.into_sorted();
        v.truncate(top_n);
        v
    }

    /// Oldest and newest file modification time in a subtree.
    pub fn subtree_date_range(&self, id: NodeId) -> (Option<i64>, Option<i64>) {
        let mut lo: Option<i64> = None;
        let mut hi: Option<i64> = None;
        for f in self.file_ids_in(id) {
            if let Some(m) = self.nodes[f as usize].modified() {
                lo = Some(lo.map_or(m, |x| x.min(m)));
                hi = Some(hi.map_or(m, |x| x.max(m)));
            }
        }
        (lo, hi)
    }

    /// Space per file type group for the whole scan, largest first. Computed once and cached.
    pub fn by_type(&self) -> Vec<TypeGroup> {
        self.type_cache.get_or_init(|| self.compute_by_type()).clone()
    }

    fn compute_by_type(&self) -> Vec<TypeGroup> {
        let mut agg = ExtAgg::default();
        for f in self.file_ids_in(0) {
            let n = &self.nodes[f as usize];
            agg.add(self.names.get(n.name), n.own_size);
        }
        let mut groups: Vec<TypeGroup> = Vec::new();
        for stat in agg.into_sorted() {
            let kind = group_of(&stat.ext);
            match groups.iter_mut().find(|g| g.group == kind) {
                Some(g) => {
                    g.bytes += stat.bytes;
                    g.files += stat.files;
                    if g.top_extensions.len() < TOP_EXTENSIONS {
                        g.top_extensions.push(stat);
                    }
                }
                None => groups.push(TypeGroup {
                    group: kind,
                    bytes: stat.bytes,
                    files: stat.files,
                    top_extensions: vec![stat],
                }),
            }
        }
        groups.retain(|g| g.bytes > 0 || g.files > 0);
        groups.sort_by_key(|g| std::cmp::Reverse(g.bytes));
        groups
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extensions() {
        let mut b = String::new();
        extension_of("Movie.MKV", &mut b);
        assert_eq!(b, "mkv");
        extension_of(".gitignore", &mut b);
        assert_eq!(b, "");
        extension_of("noext", &mut b);
        assert_eq!(b, "");
        extension_of("a.tar.gz", &mut b);
        assert_eq!(b, "gz");
    }

    #[test]
    fn groups() {
        assert_eq!(group_of("iso"), TypeGroupKind::DiskImages);
        assert_eq!(group_of("msi"), TypeGroupKind::Installers);
        assert_eq!(group_of("dll"), TypeGroupKind::Executables);
        assert_eq!(group_of("sys"), TypeGroupKind::Executables);
        assert_eq!(group_of("pagefile.sys"), TypeGroupKind::System);
        assert_eq!(group_of(""), TypeGroupKind::Other);
    }
}
