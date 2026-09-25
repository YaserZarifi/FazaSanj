//! Matches a few million synthetic paths and prints the time.
//!
//! cargo run --release -p fazasanj-rules --example bench_match [count]

use std::time::Instant;

use fazasanj_rules::RuleSet;

const NAMES: [&str; 16] = [
    "index.js", "README.md", "Cache", "src", "lib", "data.bin", "photo.jpg", "node_modules", "setup.exe",
    "config.json", "temp", "Default", "assets", "build", "report.docx", "video.mp4",
];

const BASES: [&str; 8] = [
    r"C:\Users\yaser\AppData\Local",
    r"C:\Users\yaser\AppData\Roaming",
    r"C:\Windows\System32",
    r"C:\Program Files\Vendor\App",
    r"D:\Projects\site",
    r"C:\Users\yaser\Documents",
    r"C:\Users\yaser\Downloads",
    r"C:\ProgramData\Microsoft\Windows",
];

fn main() {
    let count: usize = std::env::args().nth(1).and_then(|a| a.parse().ok()).unwrap_or(2_000_000);
    let rules = match RuleSet::load_embedded() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("rules failed to load: {e}");
            return;
        }
    };

    // Deterministic mix of shallow and deep paths, roughly like a real drive.
    let mut seed: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let mut paths = Vec::with_capacity(count);
    for _ in 0..count {
        let mut p = String::from(BASES[(next() % BASES.len() as u64) as usize]);
        let depth = 1 + next() % 7;
        for _ in 0..depth {
            p.push('\\');
            p.push_str(NAMES[(next() % NAMES.len() as u64) as usize]);
        }
        let is_dir = !p.rsplit('\\').next().unwrap_or("").contains('.');
        paths.push((p, is_dir));
    }

    let now = 1_790_000_000_000i64;
    let start = Instant::now();
    let mut hits = 0usize;
    for (p, is_dir) in &paths {
        if rules.match_path(p, *is_dir, Some(0), now).is_some() {
            hits += 1;
        }
    }
    let took = start.elapsed();
    println!(
        "{} rules, {count} paths, {hits} matched, {:.3} s ({:.0} ns per path)",
        rules.len(),
        took.as_secs_f64(),
        took.as_nanos() as f64 / count as f64
    );
}
