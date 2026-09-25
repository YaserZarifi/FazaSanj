//! Loose name matching between AppData folders and installed programs.

/// Words that say nothing about which product it is.
const NOISE: &[&str] = &[
    "inc", "ltd", "llc", "corp", "corporation", "co", "gmbh", "sa", "srl", "limited", "company", "software",
    "technologies", "technology", "the", "bv", "ag", "oy", "ab", "pty", "plc", "kg", "sas", "x64", "x86", "64bit",
    "32bit", "bit", "win64", "win32", "amd64", "setup", "installer", "portable", "version", "edition", "for",
    "windows", "and", "of",
];

/// Removed only when something else is left ("Telegram Desktop" and "Telegram" are the same app).
const SOFT_NOISE: &[&str] = &["desktop", "app", "client", "updater", "update", "launcher", "helper", "service", "beta"];

/// Tokens too common to prove a match on their own.
const WEAK_TOKENS: &[&str] = &[
    "microsoft", "update", "runtime", "redistributable", "driver", "drivers", "tools", "tool", "system", "data",
    "files", "common", "shared", "user", "cache", "helper", "manager", "studio", "visual", "plugin", "support", "sdk",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Name {
    pub tokens: Vec<String>,
    pub joined: String,
}

impl Name {
    pub fn is_empty(&self) -> bool {
        self.joined.is_empty()
    }
}

pub(crate) fn normalize(raw: &str) -> Name {
    // Split camelCase before lowercasing: "JetBrainsToolbox" and "JetBrains Toolbox" should agree.
    let mut spaced = String::with_capacity(raw.len() + 8);
    let mut prev: Option<char> = None;
    for c in raw.chars() {
        if let Some(p) = prev {
            if p.is_lowercase() && c.is_uppercase() {
                spaced.push(' ');
            }
        }
        spaced.push(c);
        prev = Some(c);
    }
    let lower = spaced.to_lowercase();
    let mut tokens: Vec<String> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .enumerate()
        // A leading number is part of the name ("7-Zip"), later ones are versions.
        .filter(|(i, t)| *i == 0 || !is_version(t))
        .map(|(_, t)| t)
        .filter(|t| !NOISE.contains(t))
        .map(strip_trailing_digits)
        .collect();
    let hard: Vec<String> = tokens.iter().filter(|t| !SOFT_NOISE.contains(&t.as_str())).cloned().collect();
    if !hard.is_empty() {
        tokens = hard;
    }
    let joined = tokens.concat();
    Name { tokens, joined }
}

/// "PyCharm2021" and "PyCharm" are the same product for our purpose.
fn strip_trailing_digits(t: &str) -> String {
    let trimmed = t.trim_end_matches(|c: char| c.is_ascii_digit());
    if trimmed.chars().any(char::is_alphabetic) {
        trimmed.to_string()
    } else {
        t.to_string()
    }
}

fn is_version(t: &str) -> bool {
    let t = t.strip_prefix('v').unwrap_or(t);
    !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit())
}

/// How sure we are that `dir` belongs to the program described by `candidate` (0..1).
pub(crate) fn similarity(dir: &Name, candidate: &Name) -> f32 {
    if dir.is_empty() || candidate.is_empty() {
        return 0.0;
    }
    if dir.joined == candidate.joined {
        return 1.0;
    }
    let (short, long) = if dir.joined.chars().count() <= candidate.joined.chars().count() {
        (dir, candidate)
    } else {
        (candidate, dir)
    };
    if short.joined.chars().count() >= 3 && is_token_run(&short.joined, &long.tokens) {
        return 0.9;
    }
    if short.joined.chars().count() >= 6 && long.joined.contains(short.joined.as_str()) {
        return 0.8;
    }
    let strong = |t: &String| t.chars().count() >= 4 && !WEAK_TOKENS.contains(&t.as_str());
    if dir.tokens.iter().filter(|t| strong(t)).any(|t| candidate.tokens.contains(t)) {
        return 0.75;
    }
    let ratio = levenshtein_ratio(&dir.joined, &candidate.joined);
    if ratio >= 0.8 {
        ratio * 0.9
    } else {
        ratio * 0.5
    }
}

/// True when `s` is a run of whole tokens ("jetbrains" in "jetbrains toolbox") that is not just
/// generic words.
fn is_token_run(s: &str, tokens: &[String]) -> bool {
    for i in 0..tokens.len() {
        let mut acc = String::new();
        for (j, t) in tokens.iter().enumerate().skip(i) {
            acc.push_str(t);
            if acc.len() > s.len() || !s.starts_with(acc.as_str()) {
                break;
            }
            if acc == s {
                if tokens[i..=j].iter().all(|t| WEAK_TOKENS.contains(&t.as_str())) {
                    break;
                }
                return true;
            }
        }
    }
    false
}

fn levenshtein_ratio(a: &str, b: &str) -> f32 {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let max = a.len().max(b.len());
    if max == 0 {
        return 1.0;
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            cur[j + 1] = (prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    1.0 - prev[b.len()] as f32 / max as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sim(a: &str, b: &str) -> f32 {
        similarity(&normalize(a), &normalize(b))
    }

    #[test]
    fn normalizing() {
        assert_eq!(normalize("JetBrains Toolbox").joined, "jetbrainstoolbox");
        assert_eq!(normalize("JetBrainsToolbox").joined, "jetbrainstoolbox");
        assert_eq!(normalize("Telegram Desktop").joined, "telegram");
        assert_eq!(normalize("7-Zip 23.01 (x64)").joined, "7zip");
        assert_eq!(normalize("Valve Corporation").joined, "valve");
        assert_eq!(normalize("Mozilla Firefox (x64 en-US)").tokens, vec!["mozilla", "firefox", "en", "us"]);
        assert_eq!(normalize("Desktop").joined, "desktop");
        assert_eq!(normalize("PyCharm2021.1").joined, "pycharm");
    }

    #[test]
    fn matching() {
        assert_eq!(sim("Telegram Desktop", "Telegram Desktop"), 1.0);
        assert!(sim("JetBrains", "JetBrains Toolbox") >= 0.9);
        assert!(sim("Mozilla", "Mozilla Firefox (x64 en-US)") >= 0.9);
        assert!(sim("obs-studio", "OBS Studio") >= 0.9);
        assert!(sim("Notepad++", "Notepad++ (64-bit x64)") >= 0.9);
        assert!(sim("discord", "Discord") >= 0.9);
        assert!(sim("Spotify", "Spotify Music") >= 0.9);
        assert!(sim("VLC", "VideoLAN") < 0.7);
        assert!(sim("OldGameStudio", "Google Chrome") < 0.5);
        assert!(sim("Data", "Microsoft Data Tools") < 0.7);
    }
}
