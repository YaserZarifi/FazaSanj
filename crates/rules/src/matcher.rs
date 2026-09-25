//! Fast path matching for millions of nodes.
//!
//! Every compiled pattern is indexed by the name of the node it can match (its last literal
//! component, a literal file name, or a file extension). For a typical path the name lookup
//! misses and we return after one hash probe without splitting or allocating anything.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

use crate::pattern::{glob_match, literal_len, match_comps, Comp};
use crate::rule::{MatchKind, Rule};

const MS_PER_DAY: i64 = 86_400_000;
/// Longest name we lowercase on the stack. Longer names can't be index keys anyway.
const KEY_BUF: usize = 96;

#[derive(Default)]
struct Fnv(u64);

impl Hasher for Fnv {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        let mut h = if self.0 == 0 { 0xcbf2_9ce4_8422_2325 } else { self.0 };
        for b in bytes {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        self.0 = h;
    }
}

type FastMap = HashMap<Box<str>, Vec<u32>, BuildHasherDefault<Fnv>>;

struct Compiled {
    rule: u32,
    kind: MatchKind,
    /// Pattern for the node path, or for its parent when `name` is set.
    comps: Vec<Comp>,
    /// Lowercase file name pattern (only for file rules with `file_patterns`).
    name: Option<String>,
    min_age_days: Option<u32>,
    /// Allowed component count of the full node path.
    min_len: usize,
    max_len: Option<usize>,
    score: (u32, bool, i32),
}

pub(crate) struct Matcher {
    pats: Vec<Compiled>,
    by_name: FastMap,
    by_ext: FastMap,
    fallback: Vec<u32>,
}

pub(crate) struct PatternError {
    pub rule: usize,
    pub pattern: String,
    pub reason: String,
}

impl Matcher {
    pub(crate) fn build(rules: &[Rule]) -> Result<Matcher, PatternError> {
        let mut m = Matcher {
            pats: Vec::new(),
            by_name: FastMap::default(),
            by_ext: FastMap::default(),
            fallback: Vec::new(),
        };
        for (ri, rule) in rules.iter().enumerate() {
            let err = |pattern: &str, reason: String| PatternError { rule: ri, pattern: pattern.to_string(), reason };
            let mut names: Vec<Option<String>> = Vec::new();
            for fp in &rule.file_patterns {
                names.push(Some(crate::pattern::check_name_pattern(fp).map_err(|r| err(fp, r))?));
            }
            if names.is_empty() {
                names.push(None);
            }
            for p in &rule.paths {
                let comps = crate::pattern::compile(p).map_err(|r| err(p, r))?;
                for name in &names {
                    m.add(ri as u32, rule, comps.clone(), name.clone());
                }
            }
        }
        Ok(m)
    }

    fn add(&mut self, rule_idx: u32, rule: &Rule, comps: Vec<Comp>, name: Option<String>) {
        let extra = usize::from(name.is_some());
        let deep = comps.iter().any(|c| matches!(c, Comp::AnyDeep));
        let fixed = comps.iter().filter(|c| !matches!(c, Comp::AnyDeep)).count() + extra;
        let score = (literal_len(&comps), name.is_some(), rule.priority);
        let key = index_key(&comps, name.as_deref());
        let idx = self.pats.len() as u32;
        self.pats.push(Compiled {
            rule: rule_idx,
            kind: rule.match_kind,
            comps,
            name,
            min_age_days: rule.min_age_days,
            min_len: fixed,
            max_len: (!deep).then_some(fixed),
            score,
        });
        match key {
            Key::Name(k) => self.by_name.entry(k.into()).or_default().push(idx),
            Key::Ext(k) => self.by_ext.entry(k.into()).or_default().push(idx),
            Key::None => self.fallback.push(idx),
        }
    }

    pub(crate) fn match_path(&self, path: &str, is_dir: bool, modified_ms: Option<i64>, now_ms: i64) -> Option<u32> {
        let path = path.strip_prefix("\\\\?\\").unwrap_or(path);
        let path = path.trim_end_matches(['\\', '/']);
        let name = path.rsplit(['\\', '/']).next().unwrap_or(path);
        if name.is_empty() {
            return None;
        }

        let mut buf = [0u8; KEY_BUF];
        let name_hits = lower(name, &mut buf).and_then(|k| self.by_name.get(k));
        let mut ext_buf = [0u8; KEY_BUF];
        let ext_hits = if is_dir {
            None
        } else {
            name.rsplit_once('.').and_then(|(_, e)| lower(e, &mut ext_buf)).and_then(|k| self.by_ext.get(k))
        };
        if name_hits.is_none() && ext_hits.is_none() && self.fallback.is_empty() {
            return None;
        }

        let mut comp_count: Option<usize> = None;
        let mut comps: Option<Vec<&str>> = None;
        let mut best: Option<(&Compiled, u32)> = None;
        let cands = name_hits
            .into_iter()
            .flatten()
            .chain(ext_hits.into_iter().flatten())
            .chain(self.fallback.iter());
        for &ci in cands {
            let Some(c) = self.pats.get(ci as usize) else { continue };
            if (c.kind == MatchKind::File) == is_dir {
                continue;
            }
            if let Some((b, _)) = best {
                if b.score >= c.score {
                    continue;
                }
            }
            if let Some(n) = &c.name {
                if !glob_match(n, name) {
                    continue;
                }
            } else if let Some(Comp::Glob(g)) = c.comps.last() {
                if !glob_match(g, name) {
                    continue;
                }
            }
            let count = *comp_count.get_or_insert_with(|| count_comps(path));
            if count < c.min_len || c.max_len.is_some_and(|m| count > m) {
                continue;
            }
            if let Some(days) = c.min_age_days {
                match modified_ms {
                    Some(m) if now_ms.saturating_sub(m) >= i64::from(days) * MS_PER_DAY => {}
                    _ => continue,
                }
            }
            let parts = comps.get_or_insert_with(|| path.split(['\\', '/']).filter(|s| !s.is_empty()).collect());
            let target: &[&str] = if c.name.is_some() { &parts[..parts.len().saturating_sub(1)] } else { parts };
            if match_comps(&c.comps, target) {
                best = Some((c, c.rule));
            }
        }
        best.map(|(_, r)| r)
    }
}

enum Key {
    Name(String),
    Ext(String),
    None,
}

fn index_key(comps: &[Comp], name: Option<&str>) -> Key {
    match name {
        Some(n) if !n.contains('*') => Key::Name(n.to_string()),
        Some(n) => match n.rsplit_once('.') {
            Some((_, ext)) if !ext.is_empty() && !ext.contains('*') => Key::Ext(ext.to_string()),
            _ => Key::None,
        },
        None => match comps.last() {
            Some(Comp::Lit(l)) => Key::Name(l.clone()),
            _ => Key::None,
        },
    }
}

fn lower<'a>(s: &str, buf: &'a mut [u8; KEY_BUF]) -> Option<&'a str> {
    let b = s.as_bytes();
    if b.len() > KEY_BUF || !s.is_ascii() {
        return None;
    }
    let out = &mut buf[..b.len()];
    for (o, i) in out.iter_mut().zip(b) {
        *o = i.to_ascii_lowercase();
    }
    std::str::from_utf8(out).ok()
}

fn count_comps(path: &str) -> usize {
    path.split(['\\', '/']).filter(|s| !s.is_empty()).count()
}
