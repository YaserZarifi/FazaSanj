//! Code projects nobody touched in a while, and the build folders that can be recreated.

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use fazasanj_model::{Bilingual, FindingDetails, HeuristicFinding, HeuristicKind, SafetyLevel};

use crate::util::{fa_num, has_part, is_system_location, months_between, MONTH_MS};
use crate::DirRecord;

/// How many entries we look at for the "last touched" date of one project.
const MAX_SOURCE_ENTRIES: usize = 50_000;
const MAX_DEPTH: usize = 12;
/// Size walk inside one build folder.
const MAX_BUILD_ENTRIES: usize = 500_000;

/// Folder names that never hold a real project of their own (packages, caches, build output).
const NOT_PROJECT_PARENTS: &[&str] = &[
    "node_modules", "target", ".venv", "venv", "site-packages", "__pycache__", ".git", "pods", ".gradle",
    "bower_components", "vendor", ".cargo", ".rustup", "$recycle.bin",
];

/// Marker file that tells what kind of project a folder is, or None.
pub fn is_project_dir(path: &Path) -> Option<&'static str> {
    let has = |name: &str| path.join(name).exists();
    if has("Cargo.toml") {
        return Some("rust");
    }
    if has("package.json") {
        return Some("node");
    }
    if has("pyproject.toml") || has("setup.py") || has("requirements.txt") || has("Pipfile") {
        return Some("python");
    }
    if has("build.gradle") || has("build.gradle.kts") || has("settings.gradle") || has("settings.gradle.kts") {
        return Some("gradle");
    }
    if has("pom.xml") {
        return Some("maven");
    }
    if has("pubspec.yaml") {
        return Some("flutter");
    }
    if has("go.mod") {
        return Some("go");
    }
    if has("Podfile") {
        return Some("cocoapods");
    }
    if has_ext(path, &["sln", "csproj", "vbproj", "fsproj"]) {
        return Some("dotnet");
    }
    if has("CMakeLists.txt") {
        return Some("cmake");
    }
    if has(".git") {
        return Some("git");
    }
    None
}

fn has_ext(dir: &Path, exts: &[&str]) -> bool {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return false;
    };
    rd.flatten().any(|e| {
        let p = e.path();
        e.file_type().is_ok_and(|t| t.is_file())
            && p.extension().is_some_and(|x| exts.iter().any(|w| x.eq_ignore_ascii_case(w)))
    })
}

/// Build output or dependency folders that the project's own tools recreate. `dir` is the folder
/// that holds it. Ambiguous names (build, dist, bin, target...) only count next to the marker
/// file of a tool that makes them.
fn is_rebuildable(parent: &Path, name: &str) -> bool {
    let has = |f: &str| parent.join(f).exists();
    match name {
        "node_modules" | ".next" | ".nuxt" | ".svelte-kit" | ".parcel-cache" | ".turbo" | ".angular" => true,
        "__pycache__" | ".pytest_cache" | ".mypy_cache" | ".ruff_cache" | ".tox" => true,
        ".gradle" => true,
        ".venv" | "venv" => parent.join(name).join("pyvenv.cfg").exists(),
        "target" => has("Cargo.toml") || has("pom.xml"),
        "build" => {
            has("build.gradle") || has("build.gradle.kts") || has("package.json") || has("CMakeLists.txt")
                || has("setup.py") || has("pyproject.toml") || has("pubspec.yaml")
        }
        "dist" => has("package.json") || has("setup.py") || has("pyproject.toml"),
        "bin" | "obj" => has_ext(parent, &["csproj", "vbproj", "fsproj"]),
        "pods" => has("Podfile"),
        _ => false,
    }
}

pub fn find_old_projects(candidate_dirs: &[DirRecord], months: u32, now_ms: i64) -> Vec<HeuristicFinding> {
    let cutoff = now_ms - i64::from(months.max(1)) * MONTH_MS;
    let mut dirs: Vec<&DirRecord> = candidate_dirs
        .iter()
        .filter(|d| !is_system_location(&d.path) && !has_part(&d.path, NOT_PROJECT_PARENTS))
        .collect();
    dirs.sort_by_key(|a| a.path.to_lowercase());

    let mut taken: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for d in dirs {
        let lower = d.path.to_lowercase().trim_end_matches('\\').to_string();
        // A project inside another candidate is covered by the outer one (monorepos, .NET).
        if taken.iter().any(|t| lower.starts_with(t.as_str()) && lower.as_bytes().get(t.len()) == Some(&b'\\')) {
            continue;
        }
        let path = Path::new(&d.path);
        let Some(kind) = is_project_dir(path) else {
            continue;
        };
        taken.push(lower);
        let survey = survey(path);
        let Some(last) = survey.last_touched else {
            continue;
        };
        if last > cutoff || survey.rebuildable_bytes == 0 {
            continue;
        }
        out.push(finding(d, kind, last, survey, months, now_ms));
    }
    out.sort_by_key(|f| std::cmp::Reverse(f.bytes));
    out
}

struct Survey {
    last_touched: Option<i64>,
    rebuildable_bytes: u64,
    rebuildable_dirs: Vec<PathBuf>,
}

/// Walks the project without following links. Build folders are measured, everything else
/// gives the "last touched" date.
fn survey(root: &Path) -> Survey {
    let mut s = Survey { last_touched: None, rebuildable_bytes: 0, rebuildable_dirs: Vec::new() };
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    let mut seen = 0usize;
    while let Some((dir, depth)) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            let Ok(ft) = e.file_type() else {
                continue;
            };
            let name = e.file_name().to_string_lossy().to_lowercase();
            if ft.is_dir() && !ft.is_symlink() {
                if is_rebuildable(&dir, &name) {
                    s.rebuildable_bytes += dir_size(&e.path());
                    s.rebuildable_dirs.push(e.path());
                    continue;
                }
                // Git internals change on fetch and gc, not when the user works.
                if name == ".git" || depth + 1 > MAX_DEPTH {
                    continue;
                }
                stack.push((e.path(), depth + 1));
            } else if seen < MAX_SOURCE_ENTRIES {
                seen += 1;
                if let Some(m) = e.metadata().ok().and_then(|m| m.modified().ok()) {
                    let ms = m.duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0);
                    s.last_touched = Some(s.last_touched.map_or(ms, |t: i64| t.max(ms)));
                }
            }
        }
    }
    s.rebuildable_dirs.sort();
    s
}

fn dir_size(root: &Path) -> u64 {
    let mut total = 0u64;
    let mut count = 0usize;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            count += 1;
            if count > MAX_BUILD_ENTRIES {
                return total;
            }
            let Ok(ft) = e.file_type() else {
                continue;
            };
            if ft.is_dir() && !ft.is_symlink() {
                stack.push(e.path());
            } else if ft.is_file() {
                total += e.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    total
}

fn finding(d: &DirRecord, kind: &str, last: i64, s: Survey, months: u32, now_ms: i64) -> HeuristicFinding {
    let age = months_between(last, now_ms);
    let limit = months.max(1);
    let confidence: f32 = if age >= limit * 4 {
        0.85
    } else if age >= limit * 2 {
        0.75
    } else {
        0.65
    };
    let names: Vec<String> = {
        let mut v: Vec<String> = s
            .rebuildable_dirs
            .iter()
            .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .collect();
        v.sort();
        v.dedup();
        v
    };
    let list = names.join(", ");
    let fa = format!(
        "این پروژهٔ {kind} حدود {} ماه است که تغییر نکرده است. پوشه‌هایی مثل {list} را ابزارهای خود پروژه دوباره می‌سازند (مثلاً با نصب دوبارهٔ وابستگی‌ها یا ساختن دوباره). به کدهای خود پروژه دست زده نمی‌شود.",
        fa_num(age)
    );
    let en = format!(
        "This {kind} project has not been changed in about {age} months. Folders like {list} are rebuilt by the project's own tools (by installing dependencies or building again). The source code itself is not touched."
    );
    HeuristicFinding {
        kind: HeuristicKind::OldProject,
        path: d.path.clone(),
        node_id: d.node_id,
        bytes: s.rebuildable_bytes,
        confidence,
        safety: SafetyLevel::ProbablySafe,
        reason: Bilingual::new(fa, en),
        details: FindingDetails::OldProject {
            project_kind: kind.to_string(),
            last_touched: Some(last),
            rebuildable_bytes: s.rebuildable_bytes,
            rebuildable_dirs: s.rebuildable_dirs.iter().map(|p| p.to_string_lossy().into_owned()).collect(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{Duration, SystemTime};

    fn write(p: &Path, n: usize) {
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, vec![1u8; n]).unwrap();
    }

    fn rec(p: &Path) -> DirRecord {
        DirRecord { node_id: Some(7), path: p.to_string_lossy().into_owned(), name: String::new(), size: 0, modified: None }
    }

    /// Sets a file's mtime so the project looks as old as we want.
    fn age_file(p: &Path, months: u64) {
        let t = SystemTime::now() - Duration::from_secs(months * 30 * 24 * 3600 + 3600 * 24 * 10);
        fs::File::options().write(true).open(p).unwrap().set_modified(t).unwrap();
    }

    fn now() -> i64 {
        crate::util::now_ms()
    }

    #[test]
    fn detects_project_kinds() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path();
        assert_eq!(is_project_dir(p), None);
        write(&p.join("x.csproj"), 1);
        assert_eq!(is_project_dir(p), Some("dotnet"));
        write(&p.join("package.json"), 1);
        assert_eq!(is_project_dir(p), Some("node"));
        write(&p.join("Cargo.toml"), 1);
        assert_eq!(is_project_dir(p), Some("rust"));
    }

    #[test]
    fn old_node_project_reports_node_modules_only() {
        let d = tempfile::tempdir().unwrap();
        let proj = d.path().join("shop");
        write(&proj.join("package.json"), 10);
        write(&proj.join("src").join("index.js"), 10);
        write(&proj.join("node_modules").join("left-pad").join("index.js"), 1000);
        write(&proj.join("node_modules").join("left-pad").join("package.json"), 500);
        // A nested package.json inside node_modules must not be its own project.
        let nested = proj.join("node_modules").join("left-pad");
        write(&proj.join("dist").join("bundle.js"), 300);
        for f in ["package.json", "src/index.js"] {
            age_file(&proj.join(f), 10);
        }
        let found = find_old_projects(&[rec(&proj), rec(&nested)], 6, now());
        assert_eq!(found.len(), 1, "{found:#?}");
        let f = &found[0];
        assert_eq!(f.bytes, 1800);
        assert_eq!(f.safety, SafetyLevel::ProbablySafe);
        assert!(f.confidence > 0.0 && f.confidence < 1.0);
        let FindingDetails::OldProject { project_kind, rebuildable_dirs, .. } = &f.details else { panic!() };
        assert_eq!(project_kind, "node");
        assert_eq!(rebuildable_dirs.len(), 2);
        assert!(f.reason.en.contains("node_modules"));
    }

    #[test]
    fn recent_projects_and_projects_without_build_output_are_skipped() {
        let d = tempfile::tempdir().unwrap();
        let fresh = d.path().join("fresh");
        write(&fresh.join("Cargo.toml"), 10);
        write(&fresh.join("target").join("debug").join("app.exe"), 5000);
        let clean = d.path().join("clean");
        write(&clean.join("Cargo.toml"), 10);
        age_file(&clean.join("Cargo.toml"), 20);
        let found = find_old_projects(&[rec(&fresh), rec(&clean)], 6, now());
        assert!(found.is_empty(), "{found:#?}");
    }

    #[test]
    fn ambiguous_names_need_their_tool() {
        let d = tempfile::tempdir().unwrap();
        // "build" and "target" here are plain folders of a git repo, not build output.
        let repo = d.path().join("notes");
        fs::create_dir_all(repo.join(".git")).unwrap();
        write(&repo.join("build").join("chapter1.md"), 100);
        write(&repo.join("target").join("goals.md"), 100);
        write(&repo.join("readme.md"), 10);
        for f in ["build/chapter1.md", "target/goals.md", "readme.md"] {
            age_file(&repo.join(f), 20);
        }
        assert!(find_old_projects(&[rec(&repo)], 6, now()).is_empty());
    }

    #[test]
    fn rust_target_and_old_age_confidence() {
        let d = tempfile::tempdir().unwrap();
        let proj = d.path().join("tool");
        write(&proj.join("Cargo.toml"), 10);
        write(&proj.join("src").join("main.rs"), 10);
        write(&proj.join("target").join("release").join("tool.exe"), 4000);
        for f in ["Cargo.toml", "src/main.rs"] {
            age_file(&proj.join(f), 30);
        }
        let found = find_old_projects(&[rec(&proj)], 6, now());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].bytes, 4000);
        assert!(found[0].confidence >= 0.85);
    }
}
