//! Path patterns: tokens, env vars, `*` and `**`, all case-insensitive.
//!
//! A pattern always starts with a token (`{localappdata}`), a classic env var
//! (`%LOCALAPPDATA%`) or `**` (anywhere on any drive). Tokens are generic on purpose:
//! `{profile}` is any `X:\Users\<name>`, so the same rules work for every user and every
//! drive, including a fake tree made by the junk generator on `T:\`.

/// Token name to template. `<drive>` is any drive, `<user>` is any single folder name.
const TOKENS: [(&str, &str); 11] = [
    ("drive", "<drive>"),
    ("profile", "<drive>\\Users\\<user>"),
    ("appdata", "<drive>\\Users\\<user>\\AppData\\Roaming"),
    ("localappdata", "<drive>\\Users\\<user>\\AppData\\Local"),
    ("locallow", "<drive>\\Users\\<user>\\AppData\\LocalLow"),
    ("temp", "<drive>\\Users\\<user>\\AppData\\Local\\Temp"),
    ("windows", "<drive>\\Windows"),
    ("programdata", "<drive>\\ProgramData"),
    ("programfiles", "<drive>\\Program Files"),
    ("programfiles_x86", "<drive>\\Program Files (x86)"),
    ("public", "<drive>\\Users\\Public"),
];

/// Classic env vars accepted in patterns and the token they stand for.
const ENV_ALIASES: [(&str, &str); 14] = [
    ("userprofile", "profile"),
    ("appdata", "appdata"),
    ("localappdata", "localappdata"),
    ("temp", "temp"),
    ("tmp", "temp"),
    ("windir", "windows"),
    ("systemroot", "windows"),
    ("programdata", "programdata"),
    ("allusersprofile", "programdata"),
    ("programfiles", "programfiles"),
    ("programw6432", "programfiles"),
    ("programfiles(x86)", "programfiles_x86"),
    ("systemdrive", "drive"),
    ("public", "public"),
];

/// Folder name used for `<user>` when making sample paths.
pub const SAMPLE_USER: &str = "Test";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Comp {
    /// `C:` style drive component.
    Drive,
    /// Lowercase ASCII literal.
    Lit(String),
    /// Lowercase ASCII with at least one `*`.
    Glob(String),
    /// `*`: exactly one component.
    Any,
    /// `**`: zero or more components.
    AnyDeep,
}

/// One pattern piece before compiling, with the original case kept (for samples).
#[derive(Debug, Clone, PartialEq, Eq)]
enum Piece<'a> {
    Drive,
    User,
    Text(&'a str),
    AnyDeep,
}

fn token_template(name: &str) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();
    TOKENS.iter().find(|(t, _)| *t == lower).map(|(_, tpl)| *tpl)
}

fn first_to_template(first: &str) -> Result<Option<&'static str>, String> {
    if let Some(tok) = first.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
        return token_template(tok).map(Some).ok_or_else(|| format!("unknown token {{{tok}}}"));
    }
    if let Some(var) = first.strip_prefix('%').and_then(|s| s.strip_suffix('%')) {
        let lower = var.to_ascii_lowercase();
        let tok = ENV_ALIASES
            .iter()
            .find(|(v, _)| *v == lower)
            .map(|(_, t)| *t)
            .ok_or_else(|| format!("unsupported env var %{var}%"))?;
        return Ok(token_template(tok));
    }
    Ok(None)
}

fn pieces(pattern: &str) -> Result<Vec<Piece<'_>>, String> {
    let mut parts = pattern.split(['\\', '/']).filter(|c| !c.is_empty());
    let first = parts.next().ok_or("empty pattern")?;
    let mut out = Vec::new();
    if let Some(tpl) = first_to_template(first)? {
        for c in tpl.split('\\') {
            out.push(match c {
                "<drive>" => Piece::Drive,
                "<user>" => Piece::User,
                t => Piece::Text(t),
            });
        }
    } else if first == "**" {
        out.push(Piece::Drive);
        out.push(Piece::AnyDeep);
    } else {
        return Err("must start with a token like {localappdata}, an env var or **".into());
    }
    for c in parts {
        if c.contains(['{', '}', '%', '?']) {
            return Err(format!("tokens and '?' are only allowed at the start ({c:?})"));
        }
        if !c.is_ascii() {
            return Err(format!("component {c:?} is not ASCII"));
        }
        if c == "." || c == ".." {
            return Err("'.' and '..' are not allowed".into());
        }
        out.push(if c == "**" { Piece::AnyDeep } else { Piece::Text(c) });
    }
    Ok(out)
}

pub(crate) fn compile(pattern: &str) -> Result<Vec<Comp>, String> {
    let comps = pieces(pattern)?
        .into_iter()
        .map(|p| match p {
            Piece::Drive => Comp::Drive,
            Piece::User => Comp::Any,
            Piece::AnyDeep => Comp::AnyDeep,
            Piece::Text("*") => Comp::Any,
            Piece::Text(t) if t.contains('*') => Comp::Glob(t.to_ascii_lowercase()),
            Piece::Text(t) => Comp::Lit(t.to_ascii_lowercase()),
        })
        .collect();
    Ok(comps)
}

/// Checks a file name pattern (`*.exe`, `thumbcache_*.db`, `hiberfil.sys`).
pub(crate) fn check_name_pattern(p: &str) -> Result<String, String> {
    if p.is_empty() || p.contains(['\\', '/', '?', '{', '%']) || !p.is_ascii() {
        return Err(format!("bad file pattern {p:?}"));
    }
    Ok(p.to_ascii_lowercase())
}

/// Case-insensitive `*` wildcard match. `pat` is lowercase ASCII.
pub(crate) fn glob_match(pat: &str, text: &str) -> bool {
    let p = pat.as_bytes();
    let t = text.as_bytes();
    let (mut pi, mut ti) = (0usize, 0usize);
    let mut star: Option<usize> = None;
    let mut mark = 0usize;
    while ti < t.len() {
        if pi < p.len() && p[pi] != b'*' && p[pi] == t[ti].to_ascii_lowercase() {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == b'*' {
            star = Some(pi);
            pi += 1;
            mark = ti;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == b'*' {
        pi += 1;
    }
    pi == p.len()
}

pub(crate) fn is_drive(c: &str) -> bool {
    let b = c.as_bytes();
    b.len() == 2 && b[0].is_ascii_alphabetic() && b[1] == b':'
}

fn comp_matches(c: &Comp, s: &str) -> bool {
    match c {
        Comp::Drive => is_drive(s),
        Comp::Lit(l) => l.eq_ignore_ascii_case(s),
        Comp::Glob(g) => glob_match(g, s),
        Comp::Any | Comp::AnyDeep => true,
    }
}

pub(crate) fn match_comps(pat: &[Comp], path: &[&str]) -> bool {
    match pat.split_first() {
        None => path.is_empty(),
        Some((Comp::AnyDeep, rest)) => (0..=path.len()).any(|i| match_comps(rest, &path[i..])),
        Some((c, rest)) => match path.split_first() {
            Some((first, tail)) => comp_matches(c, first) && match_comps(rest, tail),
            None => false,
        },
    }
}

/// Number of literal characters, used to rank how specific a pattern is.
pub(crate) fn literal_len(comps: &[Comp]) -> u32 {
    comps
        .iter()
        .map(|c| match c {
            Comp::Lit(l) => l.len() as u32,
            Comp::Glob(g) => g.bytes().filter(|b| *b != b'*').count() as u32,
            _ => 0,
        })
        .sum()
}

/// Replaces the `*` in a glob with something that reads like a real name.
fn fill_glob(g: &str) -> String {
    let chars: Vec<char> = g.chars().collect();
    let mut out = String::new();
    for (i, ch) in chars.iter().enumerate() {
        if *ch != '*' {
            out.push(*ch);
            continue;
        }
        let at_end = i + 1 == chars.len();
        let prev_ok = out.chars().last().is_some_and(|p| p.is_ascii_alphanumeric() || "._#".contains(p));
        if !(at_end && prev_ok) {
            out.push_str("sample");
        }
    }
    out
}

/// A concrete relative path (no drive) for a pattern, like `Users\Test\AppData\Local\Temp`.
/// Wildcards get plausible names so the result is a real, creatable path.
pub(crate) fn synthesize(pattern: &str) -> Result<String, String> {
    let ps = pieces(pattern)?;
    let leading_deep = matches!(ps.get(1), Some(Piece::AnyDeep));
    let mut out: Vec<String> = Vec::new();
    for (i, p) in ps.iter().enumerate() {
        match p {
            Piece::Drive => {}
            Piece::User => out.push(SAMPLE_USER.to_string()),
            Piece::AnyDeep if i == 1 && leading_deep => {
                out.push("Projects".into());
                out.push("demo".into());
            }
            Piece::AnyDeep => out.push("data".into()),
            Piece::Text("*") => out.push("Default".into()),
            Piece::Text(t) if t.contains('*') => out.push(fill_glob(t)),
            Piece::Text(t) => out.push((*t).to_string()),
        }
    }
    Ok(out.join("\\"))
}

/// A concrete file name for a file pattern.
pub(crate) fn synthesize_name(pattern: &str) -> String {
    fill_glob(pattern)
}

/// Replaces the leading token of `template` using the drive and profile of `matched_path`.
pub(crate) fn resolve_tokens(template: &str, matched_path: &str) -> Option<String> {
    let p = matched_path.strip_prefix("\\\\?\\").unwrap_or(matched_path);
    let comps: Vec<&str> = p.split(['\\', '/']).filter(|c| !c.is_empty()).collect();
    let drive = comps.first().filter(|d| is_drive(d))?;
    let user = match comps.get(1) {
        Some(u) if u.eq_ignore_ascii_case("users") => comps.get(2).copied(),
        _ => None,
    };
    let rest_start = template.find(['\\', '/']).unwrap_or(template.len());
    let (first, rest) = template.split_at(rest_start);
    let tpl = first_to_template(first).ok()??;
    if tpl.contains("<user>") && user.is_none() {
        return None;
    }
    let head = tpl.replace("<drive>", drive).replace("<user>", user.unwrap_or_default());
    Some(format!("{head}{}", rest.replace('/', "\\")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globs() {
        assert!(glob_match("*.exe", "Setup.EXE"));
        assert!(glob_match("thumbcache_*.db", "thumbcache_256.db"));
        assert!(!glob_match("thumbcache_*.db", "thumbcache_256.dbx"));
        assert!(glob_match("*canonicalgroup*", "CanonicalGroupLimited.Ubuntu_79rhkp1fndgsc"));
        assert!(glob_match("ntuser.dat*", "NTUSER.DAT"));
        assert!(!glob_match("a*b", "ac"));
    }

    #[test]
    fn compiles_tokens_and_env() {
        let a = compile("{localappdata}\\Temp").unwrap();
        let b = compile("%LOCALAPPDATA%/temp").unwrap();
        assert_eq!(a, b);
        assert_eq!(a.len(), 6);
        assert!(compile("C:\\Windows").is_err());
        assert!(compile("{nope}\\x").is_err());
        assert!(compile("{windows}\\{temp}").is_err());
    }

    #[test]
    fn matches_any_drive_and_user() {
        let p = compile("{appdata}\\Telegram Desktop\\tdata\\user_data").unwrap();
        let path = ["T:", "Users", "Test", "AppData", "Roaming", "Telegram Desktop", "tdata", "USER_DATA"];
        assert!(match_comps(&p, &path));
        let deep = compile("**\\node_modules").unwrap();
        assert!(match_comps(&deep, &["D:", "code", "a", "node_modules"]));
        assert!(match_comps(&deep, &["D:", "node_modules"]));
        assert!(!match_comps(&deep, &["D:", "node_modules", "x"]));
    }

    #[test]
    fn samples() {
        assert_eq!(synthesize("{localappdata}\\Temp").unwrap(), "Users\\Test\\AppData\\Local\\Temp");
        assert_eq!(synthesize("**\\node_modules").unwrap(), "Projects\\demo\\node_modules");
        assert_eq!(synthesize_name("*.exe"), "sample.exe");
        assert_eq!(synthesize_name("ntuser.dat*"), "ntuser.dat");
        assert_eq!(fill_glob("OneDrive - *"), "OneDrive - sample");
    }

    #[test]
    fn resolves_open_target() {
        let r = resolve_tokens("{appdata}/Telegram Desktop/Telegram.exe", "D:\\Users\\sara\\AppData\\Roaming\\x");
        assert_eq!(r.as_deref(), Some("D:\\Users\\sara\\AppData\\Roaming\\Telegram Desktop\\Telegram.exe"));
        assert_eq!(resolve_tokens("{appdata}/x.exe", "D:\\Games"), None);
    }
}
