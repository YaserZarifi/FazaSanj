//! Decides what to create, without touching the disk. The same plan drives dry runs and tests.

use std::collections::HashSet;

use fazasanj_model::Category;
use fazasanj_rules::RuleSet;

use crate::rng::Rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scale {
    /// About 4 MB, for tests.
    Tiny,
    Small,
    Medium,
    Large,
}

impl Scale {
    pub fn parse(s: &str) -> Option<Scale> {
        match s.to_ascii_lowercase().as_str() {
            "tiny" => Some(Scale::Tiny),
            "small" => Some(Scale::Small),
            "medium" => Some(Scale::Medium),
            "large" => Some(Scale::Large),
            _ => None,
        }
    }

    pub fn budget(self) -> u64 {
        const MB: u64 = 1024 * 1024;
        match self {
            Scale::Tiny => 4 * MB,
            Scale::Small => 200 * MB,
            Scale::Medium => 2048 * MB,
            Scale::Large => 10 * 1024 * MB,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Group {
    Skeleton,
    RuleExample,
    Duplicates,
    Stale,
    OldProjects,
    Orphans,
    Unknown,
}

impl Group {
    pub fn label(self) -> &'static str {
        match self {
            Group::Skeleton => "windows-like skeleton",
            Group::RuleExample => "rule examples",
            Group::Duplicates => "duplicates",
            Group::Stale => "stale files",
            Group::OldProjects => "old projects",
            Group::Orphans => "orphan app data",
            Group::Unknown => "unknown folders",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Content {
    /// Pseudo random bytes from this seed.
    Random(u64),
    /// Same bytes as the entry at this index.
    CopyOf(usize),
    /// Same bytes as the entry at this index, with the last byte changed.
    NearCopyOf(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Dir,
    File { size: u64, content: Content },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Relative to the target, with backslashes.
    pub rel: String,
    pub kind: Kind,
    /// Set the modified time this many days in the past.
    pub age_days: Option<u32>,
    pub group: Group,
    /// For rule examples: the rule this entry must be recognized as.
    pub rule_id: Option<String>,
}

#[derive(Debug, Default)]
pub struct Plan {
    pub entries: Vec<Entry>,
    seen: HashSet<String>,
}

impl Plan {
    fn push(&mut self, e: Entry) -> Option<usize> {
        let key = e.rel.to_ascii_lowercase();
        if !self.seen.insert(key) {
            return None;
        }
        self.entries.push(e);
        Some(self.entries.len() - 1)
    }

    fn dir(&mut self, rel: &str, group: Group, age: Option<u32>) {
        self.push(Entry { rel: rel.to_string(), kind: Kind::Dir, age_days: age, group, rule_id: None });
    }

    fn file(&mut self, rel: &str, size: u64, content: Content, group: Group, age: Option<u32>) -> Option<usize> {
        self.push(Entry { rel: rel.to_string(), kind: Kind::File { size, content }, age_days: age, group, rule_id: None })
    }

    pub fn total_bytes(&self) -> u64 {
        self.entries
            .iter()
            .map(|e| match e.kind {
                Kind::File { size, .. } => size,
                Kind::Dir => 0,
            })
            .sum()
    }
}

/// Builds the full plan. Same rules, scale and seed always give the same plan.
pub fn build(rules: &RuleSet, scale: Scale, seed: u64) -> Plan {
    let mut rng = Rng::new(seed);
    let mut plan = Plan::default();
    let budget = scale.budget();

    skeleton(&mut plan, &mut rng, budget / 50);
    rule_examples(&mut plan, &mut rng, rules, budget * 60 / 100);
    duplicates(&mut plan, &mut rng, budget / 10);
    stale(&mut plan, &mut rng, budget / 10);
    old_projects(&mut plan, &mut rng, budget * 12 / 100);
    orphans(&mut plan, &mut rng, budget * 3 / 100);
    unknown(&mut plan, &mut rng, budget * 3 / 100);
    plan
}

fn vary(rng: &mut Rng, base: u64) -> u64 {
    // 50% to 150% of base, at least one byte.
    (base / 2 + rng.below(base.max(1))).max(1)
}

fn skeleton(plan: &mut Plan, rng: &mut Rng, budget: u64) {
    let each = (budget / 12).max(1024);
    let files = [
        r"Windows\System32\drivers\etc\hosts",
        r"Windows\System32\kernel32.dll",
        r"Windows\SysWOW64\kernel32.dll",
        r"Windows\explorer.exe",
        r"Program Files\FakeApp\fakeapp.exe",
        r"Program Files (x86)\OldFakeApp\app.exe",
        r"ProgramData\FakeVendor\settings.ini",
        r"Users\Test\Documents\notes.txt",
        r"Users\Test\Desktop\todo.txt",
        r"Users\Test\Pictures\wallpaper.jpg",
        r"Users\Test\Videos\birthday.mp4",
        r"Users\Test\Music\song.mp3",
    ];
    for f in files {
        let s = rng.next_u64();
        plan.file(f, vary(rng, each), Content::Random(s), Group::Skeleton, None);
    }
    // A fresh installer that must NOT be picked up by the old installer rule.
    let s = rng.next_u64();
    plan.file(r"Users\Test\Downloads\new-setup.exe", vary(rng, each), Content::Random(s), Group::Skeleton, Some(5));
    plan.dir(r"Users\Test\AppData\LocalLow", Group::Skeleton, None);
    plan.dir(r"Users\Public\Documents", Group::Skeleton, None);
}

fn rule_examples(plan: &mut Plan, rng: &mut Rng, rules: &RuleSet, budget: u64) {
    let mut items = Vec::new();
    for i in 0..rules.len() as u32 {
        let Some(rule) = rules.get(i) else { continue };
        let heavy = matches!(rule.category, Category::Virtualization | Category::System);
        for ex in rules.examples(i) {
            items.push((rule.id.clone(), ex, if heavy { 3u64 } else { 1 }));
        }
    }
    let weight: u64 = items.iter().map(|(_, _, w)| *w).sum::<u64>().max(1);
    for (id, ex, w) in items {
        let size = vary(rng, budget * w / weight);
        let age = ex.min_age_days.map(|d| d + 30);
        if ex.is_dir {
            let idx = plan.push(Entry {
                rel: ex.rel_path.clone(),
                kind: Kind::Dir,
                age_days: None,
                group: Group::RuleExample,
                rule_id: Some(id),
            });
            if idx.is_none() {
                continue;
            }
            let n = 1 + rng.below(3);
            for k in 0..n {
                let s = rng.next_u64();
                let rel = format!(r"{}\blob_{k:03}.bin", ex.rel_path);
                plan.file(&rel, (size / n).max(1), Content::Random(s), Group::RuleExample, None);
            }
        } else {
            let s = rng.next_u64();
            plan.push(Entry {
                rel: ex.rel_path.clone(),
                kind: Kind::File { size, content: Content::Random(s) },
                age_days: age,
                group: Group::RuleExample,
                rule_id: Some(id),
            });
        }
    }
}

fn duplicates(plan: &mut Plan, rng: &mut Rng, budget: u64) {
    let each = (budget / 12).max(1024);
    for n in 1..=3 {
        let s = rng.next_u64();
        let size = vary(rng, each);
        let orig = format!(r"Users\Test\Pictures\Trip\IMG_{n:04}.jpg");
        let Some(src) = plan.file(&orig, size, Content::Random(s), Group::Duplicates, Some(200)) else { continue };
        plan.file(&format!(r"Users\Test\Desktop\backup\IMG_{n:04}.jpg"), size, Content::CopyOf(src), Group::Duplicates, Some(150));
        plan.file(
            &format!(r"Users\Test\Documents\old phone\IMG_{n:04} (1).jpg"),
            size,
            Content::CopyOf(src),
            Group::Duplicates,
            Some(100),
        );
    }
    for n in 10..=11 {
        let s = rng.next_u64();
        let size = vary(rng, each);
        let orig = format!(r"Users\Test\Pictures\Trip\IMG_{n:04}.jpg");
        let Some(src) = plan.file(&orig, size, Content::Random(s), Group::Duplicates, Some(200)) else { continue };
        plan.file(
            &format!(r"Users\Test\Desktop\backup\IMG_{n:04}.jpg"),
            size,
            Content::NearCopyOf(src),
            Group::Duplicates,
            Some(150),
        );
    }
}

fn stale(plan: &mut Plan, rng: &mut Rng, budget: u64) {
    let each = (budget / 6).max(1024);
    for n in 1..=6 {
        let s = rng.next_u64();
        let rel = format!(r"Users\Test\Documents\old reports\report-2019-{n:02}.pdf");
        plan.file(&rel, vary(rng, each), Content::Random(s), Group::Stale, Some(730 + n * 3));
    }
}

fn old_projects(plan: &mut Plan, rng: &mut Rng, budget: u64) {
    let each = (budget / 10).max(1024);
    let age = Some(420);
    let small = |plan: &mut Plan, rng: &mut Rng, rel: &str| {
        let s = rng.next_u64();
        plan.file(rel, 200 + rng.below(2000), Content::Random(s), Group::OldProjects, age);
    };
    let web = r"Users\Test\Projects\old-shop";
    for f in [r"package.json", r"src\index.js", r"src\cart.js", r".git\HEAD", r".git\config"] {
        small(plan, rng, &format!(r"{web}\{f}"));
    }
    for f in [r"node_modules\left-pad\index.js", r"node_modules\react\cjs\react.development.js", r"node_modules\.bin\vite"] {
        let s = rng.next_u64();
        plan.file(&format!(r"{web}\{f}"), vary(rng, each), Content::Random(s), Group::OldProjects, age);
    }
    let s = rng.next_u64();
    plan.file(&format!(r"{web}\dist\bundle.js"), vary(rng, each), Content::Random(s), Group::OldProjects, age);
    let s = rng.next_u64();
    plan.file(&format!(r"{web}\.git\objects\pack\pack-1.pack"), vary(rng, each), Content::Random(s), Group::OldProjects, age);

    let rs = r"Users\Test\Projects\old-rust-tool";
    for f in [r"Cargo.toml", r"Cargo.lock", r"src\main.rs", r".git\HEAD"] {
        small(plan, rng, &format!(r"{rs}\{f}"));
    }
    for f in [r"target\debug\old-rust-tool.exe", r"target\debug\deps\libserde-1a2b.rlib", r"target\release\old-rust-tool.exe"] {
        let s = rng.next_u64();
        plan.file(&format!(r"{rs}\{f}"), vary(rng, each), Content::Random(s), Group::OldProjects, age);
    }
    for d in [web, rs] {
        plan.dir(d, Group::OldProjects, age);
    }
}

fn orphans(plan: &mut Plan, rng: &mut Rng, budget: u64) {
    let each = (budget / 5).max(1024);
    let files = [
        r"Users\Test\AppData\Roaming\OldToolThatIsGone\settings.json",
        r"Users\Test\AppData\Roaming\OldToolThatIsGone\data\cache.bin",
        r"Users\Test\AppData\Local\RetiredPhotoApp\thumbs\thumbs.db.bin",
        r"Users\Test\AppData\Local\RetiredPhotoApp\library.sqlite",
        r"ProgramData\DefunctVendor\Updater\payload.bin",
    ];
    for f in files {
        let s = rng.next_u64();
        plan.file(f, vary(rng, each), Content::Random(s), Group::Orphans, Some(400));
    }
}

fn unknown(plan: &mut Plan, rng: &mut Rng, budget: u64) {
    let each = (budget / 6).max(1024);
    for _ in 0..3 {
        let name = format!("{:08x}", rng.next_u64() as u32);
        for k in 0..2 {
            let s = rng.next_u64();
            let rel = format!(r"Stuff\{name}\blob_{k}.dat");
            plan.file(&rel, vary(rng, each), Content::Random(s), Group::Unknown, None);
        }
    }
}
