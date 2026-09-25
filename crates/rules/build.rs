//! Embeds every `rules/*.json` file so the app needs no resource files at runtime.

use std::fmt::Write as _;
use std::path::PathBuf;

fn main() {
    let dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default()).join("rules");
    println!("cargo:rerun-if-changed={}", dir.display());

    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.path()).collect())
        .unwrap_or_default();
    files.retain(|p| p.extension().is_some_and(|e| e == "json"));
    files.sort();

    let mut out = String::from("pub(crate) static EMBEDDED: &[(&str, &str)] = &[\n");
    for f in &files {
        println!("cargo:rerun-if-changed={}", f.display());
        let name = f.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let _ = writeln!(out, "    ({name:?}, include_str!({:?})),", f.display().to_string());
    }
    out.push_str("];\n");

    let dest = PathBuf::from(std::env::var("OUT_DIR").unwrap_or_default()).join("embedded.rs");
    if let Err(e) = std::fs::write(&dest, out) {
        panic!("cannot write {}: {e}", dest.display());
    }
}
