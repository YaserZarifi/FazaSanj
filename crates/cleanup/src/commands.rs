//! The only system commands and settings pages the engine will ever start.
//!
//! Nothing from a rule or the UI is passed to a shell as is. Input is matched against the
//! known forms below and the engine builds the real command line itself.

use std::path::{Path, PathBuf};

/// Program and arguments as a rule describes them, before validation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
}

impl CommandSpec {
    pub fn new(program: impl Into<String>, args: &[&str]) -> Self {
        Self { program: program.into(), args: args.iter().map(|a| a.to_string()).collect() }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnownCommand {
    /// `dism.exe /Online /Cleanup-Image /StartComponentCleanup`
    DismComponentCleanup,
    /// `powercfg.exe /hibernate off`
    HibernateOff,
    /// `powercfg.exe /h /type reduced`
    HibernateReduced,
    /// `vssadmin.exe delete shadows /for=C: /oldest /quiet`
    DeleteOldestShadow { drive: char },
    /// `cleanmgr.exe /d C`
    Cleanmgr { drive: char },
    /// `cleanmgr.exe /sagerun:N`
    CleanmgrSagerun { n: u16 },
}

impl KnownCommand {
    /// Maps a rule's program and args to a known command. `<drive>` style placeholders take the
    /// drive of `target_path`.
    pub fn from_spec(spec: &CommandSpec, target_path: &str) -> Option<Self> {
        let program = spec.program.rsplit(['\\', '/']).next().unwrap_or("");
        let drive = drive_of(target_path);
        let mut tokens = vec![program.to_string()];
        for a in &spec.args {
            let low = a.trim().to_ascii_lowercase();
            let replaced = match (low.as_str(), drive) {
                ("<drive>" | "{drive}" | "%drive%" | "%systemdrive%", Some(d)) => format!("{d}:"),
                (s, Some(d)) if s.starts_with("/for=") && is_placeholder(&s[5..]) => format!("/for={d}:"),
                _ => low,
            };
            tokens.push(replaced);
        }
        Self::from_tokens(&tokens)
    }

    /// Reads back a command line produced by [`KnownCommand::display`]. Anything else is refused.
    pub fn parse(line: &str) -> Option<Self> {
        let tokens: Vec<String> = line.split_whitespace().map(str::to_string).collect();
        Self::from_tokens(&tokens)
    }

    /// Official tool for well known system items, when a rule gives no command data.
    pub fn infer_from_path(path: &str) -> Option<Self> {
        let p = fazasanj_safety::normalize(Path::new(path));
        let drive = drive_of(&p)?;
        let rest = p.get(2..).unwrap_or("");
        match rest {
            "\\hiberfil.sys" => Some(KnownCommand::HibernateOff),
            "\\system volume information" => Some(KnownCommand::DeleteOldestShadow { drive }),
            _ if rest.ends_with("\\winsxs") => Some(KnownCommand::DismComponentCleanup),
            _ => None,
        }
    }

    fn from_tokens(tokens: &[String]) -> Option<Self> {
        let low: Vec<String> = tokens.iter().map(|t| t.to_ascii_lowercase()).collect();
        let (first, rest) = low.split_first()?;
        let program = first.strip_suffix(".exe").unwrap_or(first);
        let rest: Vec<&str> = rest.iter().map(String::as_str).collect();
        match (program, rest.as_slice()) {
            ("dism", ["/online", "/cleanup-image", "/startcomponentcleanup"]) => {
                Some(KnownCommand::DismComponentCleanup)
            }
            ("powercfg", ["/hibernate" | "-hibernate" | "/h" | "-h", "off"]) => Some(KnownCommand::HibernateOff),
            ("powercfg", ["/hibernate" | "-hibernate" | "/h" | "-h", "/type" | "-type", "reduced"]) => {
                Some(KnownCommand::HibernateReduced)
            }
            ("vssadmin", ["delete", "shadows", for_arg, "/oldest"])
            | ("vssadmin", ["delete", "shadows", for_arg, "/oldest", "/quiet"]) => {
                let d = for_arg.strip_prefix("/for=").and_then(parse_drive)?;
                Some(KnownCommand::DeleteOldestShadow { drive: d })
            }
            ("cleanmgr", ["/d", d]) => parse_drive(d).map(|drive| KnownCommand::Cleanmgr { drive }),
            ("cleanmgr", [s]) => {
                let n = s.strip_prefix("/sagerun:")?;
                if n.is_empty() || n.len() > 4 || !n.bytes().all(|b| b.is_ascii_digit()) {
                    return None;
                }
                n.parse().ok().map(|n| KnownCommand::CleanmgrSagerun { n })
            }
            _ => None,
        }
    }

    pub fn program(&self) -> &'static str {
        match self {
            KnownCommand::DismComponentCleanup => "dism.exe",
            KnownCommand::HibernateOff | KnownCommand::HibernateReduced => "powercfg.exe",
            KnownCommand::DeleteOldestShadow { .. } => "vssadmin.exe",
            KnownCommand::Cleanmgr { .. } | KnownCommand::CleanmgrSagerun { .. } => "cleanmgr.exe",
        }
    }

    pub fn args(&self) -> Vec<String> {
        let v: Vec<String> = match self {
            KnownCommand::DismComponentCleanup => {
                vec!["/Online".into(), "/Cleanup-Image".into(), "/StartComponentCleanup".into()]
            }
            KnownCommand::HibernateOff => vec!["/hibernate".into(), "off".into()],
            KnownCommand::HibernateReduced => vec!["/h".into(), "/type".into(), "reduced".into()],
            KnownCommand::DeleteOldestShadow { drive } => {
                // Without /quiet vssadmin waits for a Y/N answer in a window the user may not see.
                vec!["delete".into(), "shadows".into(), format!("/for={drive}:"), "/oldest".into(), "/quiet".into()]
            }
            KnownCommand::Cleanmgr { drive } => vec!["/d".into(), drive.to_string()],
            KnownCommand::CleanmgrSagerun { n } => vec![format!("/sagerun:{n}")],
        };
        v
    }

    /// Human readable command line, also the form stored in the plan.
    pub fn display(&self) -> String {
        let mut s = self.program().to_string();
        for a in self.args() {
            s.push(' ');
            s.push_str(&a);
        }
        s
    }

    pub fn needs_admin(&self) -> bool {
        !matches!(self, KnownCommand::Cleanmgr { .. } | KnownCommand::CleanmgrSagerun { .. })
    }

    /// cleanmgr has its own window, the others are console tools we keep hidden.
    pub fn shows_ui(&self) -> bool {
        matches!(self, KnownCommand::Cleanmgr { .. } | KnownCommand::CleanmgrSagerun { .. })
    }

    /// Drive the command frees space on, for the free space numbers.
    pub fn drive(&self) -> Option<char> {
        match self {
            KnownCommand::DeleteOldestShadow { drive } | KnownCommand::Cleanmgr { drive } => Some(*drive),
            _ => None,
        }
    }
}

fn is_placeholder(s: &str) -> bool {
    matches!(s, "<drive>" | "{drive}" | "%drive%" | "%systemdrive%")
}

fn parse_drive(s: &str) -> Option<char> {
    let s = s.strip_suffix(':').unwrap_or(s);
    let mut chars = s.chars();
    let c = chars.next()?;
    if chars.next().is_some() || !c.is_ascii_alphabetic() {
        return None;
    }
    Some(c.to_ascii_uppercase())
}

fn drive_of(path: &str) -> Option<char> {
    let p = path.strip_prefix("\\\\?\\").unwrap_or(path);
    let b = p.as_bytes();
    (b.len() >= 2 && b[1] == b':' && b[0].is_ascii_alphabetic()).then(|| (b[0] as char).to_ascii_uppercase())
}

/// Settings pages and tools `open_app_setting` may open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenTarget {
    Settings(String),
    SystemTool(&'static str),
}

const OPEN_TOOLS: [&str; 6] = [
    "SystemPropertiesProtection.exe",
    "SystemPropertiesAdvanced.exe",
    "SystemPropertiesPerformance.exe",
    "cleanmgr.exe",
    "appwiz.cpl",
    "diskmgmt.msc",
];

impl OpenTarget {
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if let Some(page) = s.strip_prefix("ms-settings:") {
            let ok = page.len() <= 64 && page.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
            return ok.then(|| OpenTarget::Settings(format!("ms-settings:{page}")));
        }
        OPEN_TOOLS.iter().find(|t| t.eq_ignore_ascii_case(s)).map(|t| OpenTarget::SystemTool(t))
    }

    pub fn display(&self) -> String {
        match self {
            OpenTarget::Settings(s) => s.clone(),
            OpenTarget::SystemTool(t) => t.to_string(),
        }
    }

    /// What goes to ShellExecute: the URI, or the full path inside System32.
    pub fn resolve(&self, system_dir: &Path) -> PathBuf {
        match self {
            OpenTarget::Settings(s) => PathBuf::from(s),
            OpenTarget::SystemTool(t) => system_dir.join(t),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_forms_round_trip() {
        let all = [
            KnownCommand::DismComponentCleanup,
            KnownCommand::HibernateOff,
            KnownCommand::HibernateReduced,
            KnownCommand::DeleteOldestShadow { drive: 'C' },
            KnownCommand::Cleanmgr { drive: 'D' },
            KnownCommand::CleanmgrSagerun { n: 7 },
        ];
        for c in all {
            assert_eq!(KnownCommand::parse(&c.display()), Some(c), "{}", c.display());
        }
        assert_eq!(
            KnownCommand::DismComponentCleanup.display(),
            "dism.exe /Online /Cleanup-Image /StartComponentCleanup"
        );
    }

    #[test]
    fn spec_mapping() {
        let s = CommandSpec::new("dism", &["/Online", "/Cleanup-Image", "/StartComponentCleanup"]);
        assert_eq!(KnownCommand::from_spec(&s, r"C:\Windows\WinSxS"), Some(KnownCommand::DismComponentCleanup));
        let s = CommandSpec::new(r"C:\Windows\System32\powercfg.exe", &["-h", "off"]);
        assert_eq!(KnownCommand::from_spec(&s, r"C:\hiberfil.sys"), Some(KnownCommand::HibernateOff));
        let s = CommandSpec::new("vssadmin", &["delete", "shadows", "/for=<drive>", "/oldest"]);
        assert_eq!(
            KnownCommand::from_spec(&s, r"E:\System Volume Information"),
            Some(KnownCommand::DeleteOldestShadow { drive: 'E' })
        );
        let s = CommandSpec::new("cleanmgr.exe", &["/d", "<drive>"]);
        assert_eq!(KnownCommand::from_spec(&s, r"D:\x"), Some(KnownCommand::Cleanmgr { drive: 'D' }));
    }

    #[test]
    fn refuses_anything_else() {
        let bad = [
            "cmd.exe /c del C:\\",
            "dism.exe /Online /Cleanup-Image /StartComponentCleanup /ResetBase",
            "dism.exe /Online /Cleanup-Image /StartComponentCleanup & calc",
            "vssadmin delete shadows /all /quiet",
            "vssadmin delete shadows /for=CC: /oldest",
            "powercfg /hibernate on",
            "cleanmgr /sagerun:1;calc",
            "powershell -c rm -r C:\\",
            "",
        ];
        for b in bad {
            assert_eq!(KnownCommand::parse(b), None, "{b}");
        }
        let s = CommandSpec::new("rundll32", &["x"]);
        assert_eq!(KnownCommand::from_spec(&s, r"C:\x"), None);
        let s = CommandSpec::new("cleanmgr", &["/d", "<drive>"]);
        assert_eq!(KnownCommand::from_spec(&s, r"\\server\share\x"), None);
    }

    #[test]
    fn infers_system_items() {
        assert_eq!(KnownCommand::infer_from_path(r"C:\hiberfil.sys"), Some(KnownCommand::HibernateOff));
        assert_eq!(KnownCommand::infer_from_path(r"C:\Windows\WinSxS"), Some(KnownCommand::DismComponentCleanup));
        assert_eq!(
            KnownCommand::infer_from_path(r"D:\System Volume Information"),
            Some(KnownCommand::DeleteOldestShadow { drive: 'D' })
        );
        assert_eq!(KnownCommand::infer_from_path(r"C:\Users\x\hiberfil.sys"), None);
    }

    #[test]
    fn open_targets() {
        assert_eq!(
            OpenTarget::parse("ms-settings:storagesense"),
            Some(OpenTarget::Settings("ms-settings:storagesense".into()))
        );
        assert!(OpenTarget::parse("ms-settings:x&calc").is_none());
        assert!(OpenTarget::parse("ms-settings:../../x").is_none());
        assert_eq!(
            OpenTarget::parse("systempropertiesprotection.exe"),
            Some(OpenTarget::SystemTool("SystemPropertiesProtection.exe"))
        );
        assert!(OpenTarget::parse("calc.exe").is_none());
        assert!(OpenTarget::parse(r"C:\evil\SystemPropertiesProtection.exe").is_none());
        assert!(OpenTarget::parse("https://example.com").is_none());
    }
}
