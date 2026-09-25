//! Scan a folder and print totals and timing. Read only.
//!
//! cargo run -p fazasanj-scan --release --example bench -- C:\ [--fast] [--helper path]

use std::path::PathBuf;
use std::time::Instant;

use fazasanj_model::ScanMode;
use fazasanj_scan::{run_scan, ScanOptions};

fn gb(b: u64) -> f64 {
    b as f64 / (1u64 << 30) as f64
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut root = PathBuf::from(r"C:\");
    let mut mode = ScanMode::Normal;
    let mut helper = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--fast" => mode = ScanMode::Fast,
            "--helper" => helper = args.next().map(PathBuf::from),
            _ => root = PathBuf::from(a),
        }
    }

    let mut opts = ScanOptions::new(&root, mode);
    opts.helper_path = helper;
    let started = Instant::now();
    let out = match run_scan(opts) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("scan failed: {e}");
            std::process::exit(1);
        }
    };
    let secs = started.elapsed().as_secs_f64();
    let t = &out.tree;
    let s = t.summary();
    println!("root         {}", t.root_path());
    println!("scanner      {:?} (fallback: {:?} {:?})", out.scanner_used, out.fallback_reason, out.fallback_detail);
    println!("files        {}", s.files);
    println!("folders      {}", s.dirs);
    println!("bytes        {} ({:.2} GB)", s.total_bytes, gb(s.total_bytes));
    println!("time         {secs:.2} s");
    println!("nodes        {} ({} bytes/node incl. names)", t.len(), t.memory_bytes() / t.len().max(1));
    println!("hard links   {} extra links not counted", s.hardlink_dups);
    println!("cloud only   {}", s.cloud_only);
    println!("no access    {} folders", out.access_denied.len());

    for p in [r"C:\Windows\WinSxS", r"C:\Windows\System32", r"C:\Windows"] {
        if let Some(id) = t.find_by_path(p) {
            let n = t.node_info(id).map_or(0, |i| i.size);
            println!("{p:<22} {:.2} GB", gb(n));
        }
    }

    let t0 = Instant::now();
    let mut visited = 0u64;
    t.visit_dirs_and_files(&mut |_| {
        visited += 1;
        fazasanj_scan::Visit::Continue
    });
    println!("visit all    {visited} nodes in {:.0} ms", t0.elapsed().as_secs_f64() * 1000.0);
    let t0 = Instant::now();
    let _ = t.by_type();
    let _ = t.largest_files(100);
    let _ = t.category_totals();
    println!("queries      by_type + largest + category totals in {:.0} ms", t0.elapsed().as_secs_f64() * 1000.0);

    if let Ok(space) = fazasanj_platform::disk_space(&root) {
        let used = space.total - space.free;
        println!(
            "drive used   {:.2} GB, scan counted {:.1}% of it",
            gb(used),
            s.total_bytes as f64 * 100.0 / used.max(1) as f64
        );
    }
}
