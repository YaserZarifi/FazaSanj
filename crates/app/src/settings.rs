//! Settings and app level actions.

use fazasanj_model::{AppSettings, Language};

use crate::{lock, store_err, ApiResult, App};

impl App {
    /// Stored settings, or the defaults when nothing (or nothing readable) is stored.
    pub fn settings(&self) -> AppSettings {
        self.store.get_settings().unwrap_or_default()
    }

    /// Cleans up and saves the settings, then returns what was saved.
    pub fn set_settings(&self, settings: AppSettings) -> ApiResult<AppSettings> {
        let clean = sanitize(settings);
        self.store.set_settings(&clean).map_err(store_err)?;
        Ok(clean)
    }

    /// Forgets everything: settings, snapshots, history, AI cache, API keys and open scans.
    pub fn reset_everything(&self) -> ApiResult<()> {
        for p in fazasanj_ai::ALL_PROVIDERS {
            if let Err(e) = fazasanj_ai::keys::delete(p) {
                log::warn!("could not delete key for {p:?}: {e}");
            }
        }
        lock(&self.scans).clear();
        lock(&self.plans).clear();
        self.store.reset_everything().map_err(store_err)
    }

    pub fn language(&self) -> Language {
        self.settings().language
    }
}

fn sanitize(mut s: AppSettings) -> AppSettings {
    s.stale_months = s.stale_months.clamp(1, 120);
    s.old_project_months = s.old_project_months.clamp(1, 120);
    s.low_space_threshold_gb = s.low_space_threshold_gb.clamp(1, 1000);
    let mut paths: Vec<String> = s
        .excluded_paths
        .into_iter()
        .map(|p| p.trim().trim_end_matches(['\\', '/']).to_string())
        .filter(|p| !p.is_empty())
        .collect();
    paths.sort_by_key(|p| p.to_lowercase());
    paths.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    s.excluded_paths = paths;
    s.dev_sandbox = s.dev_sandbox.map(|p| p.trim().to_string()).filter(|p| !p.is_empty());
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_clamps_and_dedups() {
        let s = AppSettings {
            stale_months: 0,
            excluded_paths: vec!["D:\\Games\\".into(), "d:\\games".into(), "  ".into()],
            dev_sandbox: Some("  ".into()),
            ..AppSettings::default()
        };
        let s = sanitize(s);
        assert_eq!(s.stale_months, 1);
        assert_eq!(s.excluded_paths, vec!["D:\\Games".to_string()]);
        assert_eq!(s.dev_sandbox, None);
    }
}
