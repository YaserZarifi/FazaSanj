//! The rule as it is written in the JSON files.

use fazasanj_model::{Bilingual, Category, CleanupMethod, SafetyLevel};
use serde::{Deserialize, Serialize};

/// Programs a `command` rule may run. Anything else is rejected by validation.
pub const COMMAND_ALLOW_LIST: [&str; 4] = ["dism", "powercfg", "vssadmin", "cleanmgr"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchKind {
    /// The folder itself is the target.
    Folder,
    /// The folder is the target, but cleanup only removes what is inside.
    Contents,
    /// Single files. With `file_patterns` the path pattern matches the parent folder.
    File,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleCommand {
    /// One of [`COMMAND_ALLOW_LIST`].
    pub program: String,
    /// `{drive}` is replaced with the drive of the matched item, like `C:`.
    #[serde(default)]
    pub args: Vec<String>,
    /// The program shows its own window (cleanmgr), so the app should not wait for output.
    #[serde(default)]
    pub own_ui: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub id: String,
    pub category: Category,
    pub paths: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub file_patterns: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_age_days: Option<u32>,
    #[serde(rename = "match", default = "default_match")]
    pub match_kind: MatchKind,
    /// When false, only the matched folder itself is labeled and the scan keeps walking into it.
    /// Used for containers like Program Files that hold other known items.
    #[serde(default = "default_true")]
    pub inherit: bool,
    #[serde(default)]
    pub priority: i32,
    pub title: Bilingual,
    pub why_big: Bilingual,
    pub if_deleted: Bilingual,
    pub safety: SafetyLevel,
    pub method: CleanupMethod,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<RuleCommand>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instructions: Option<Bilingual>,
    /// `ms-settings:` URI or an exe (name or path pattern with tokens) to open.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_target: Option<String>,
    pub needs_admin: bool,
    #[serde(default)]
    pub permanent_ok: bool,
    /// Concrete sample paths (with tokens, no wildcards) for tests and the junk generator.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

fn default_match() -> MatchKind {
    MatchKind::Folder
}

fn default_true() -> bool {
    true
}

impl Rule {
    /// True when the scan should stop walking into a folder matched by this rule
    /// (everything below it belongs to the rule).
    pub fn stops_descent(&self) -> bool {
        self.match_kind != MatchKind::File && self.inherit
    }

    /// Command arguments for a matched item, with `{drive}` filled in.
    pub fn command_args(&self, matched_path: &str) -> Vec<String> {
        let drive = drive_of(matched_path).unwrap_or_default();
        self.command
            .as_ref()
            .map(|c| c.args.iter().map(|a| a.replace("{drive}", &drive)).collect())
            .unwrap_or_default()
    }

    /// Readable command line for the UI, like `powercfg /h off`.
    pub fn command_line(&self, matched_path: &str) -> Option<String> {
        let c = self.command.as_ref()?;
        let mut parts = vec![c.program.clone()];
        parts.extend(self.command_args(matched_path));
        Some(parts.join(" "))
    }

    /// `open_target` with tokens like `{appdata}` resolved against the matched path's profile.
    pub fn resolve_open_target(&self, matched_path: &str) -> Option<String> {
        let t = self.open_target.as_ref()?;
        if !t.contains('{') {
            return Some(t.clone());
        }
        crate::pattern::resolve_tokens(t, matched_path)
    }
}

/// `C:` from `C:\anything`.
pub(crate) fn drive_of(path: &str) -> Option<String> {
    let p = path.strip_prefix("\\?\\").unwrap_or(path);
    let b = p.as_bytes();
    (b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':').then(|| p[..2].to_ascii_uppercase())
}
