//! Small key/value storage and the app settings (stored as one JSON value).

use fazasanj_model::AppSettings;
use rusqlite::{params, OptionalExtension};
use serde_json::{Map, Value};

use crate::{Result, Store};

const SETTINGS_KEY: &str = "app_settings";

impl Store {
    pub fn get_value(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn();
        let v = conn
            .query_row("SELECT value FROM kv WHERE key = ?1", [key], |r| r.get(0))
            .optional()?;
        Ok(v)
    }

    pub fn set_value(&self, key: &str, value: &str) -> Result<()> {
        self.conn().execute(
            "INSERT INTO kv (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn delete_value(&self, key: &str) -> Result<()> {
        self.conn().execute("DELETE FROM kv WHERE key = ?1", [key])?;
        Ok(())
    }

    /// Never fails on bad stored data: missing or unreadable fields fall back to defaults.
    pub fn get_settings(&self) -> Result<AppSettings> {
        Ok(match self.get_value(SETTINGS_KEY)? {
            Some(raw) => settings_from_json(&raw),
            None => AppSettings::default(),
        })
    }

    pub fn set_settings(&self, settings: &AppSettings) -> Result<()> {
        let raw = serde_json::to_string(settings)?;
        self.set_value(SETTINGS_KEY, &raw)
    }
}

/// Unknown fields are ignored by serde already. A field with a value this version cannot read
/// (say a theme added later) would fail the whole struct, so fields are applied one by one
/// and the bad ones keep their default.
fn settings_from_json(raw: &str) -> AppSettings {
    if let Ok(s) = serde_json::from_str::<AppSettings>(raw) {
        return s;
    }
    let Ok(Value::Object(stored)) = serde_json::from_str::<Value>(raw) else {
        return AppSettings::default();
    };
    let Ok(Value::Object(mut merged)) = serde_json::to_value(AppSettings::default()) else {
        return AppSettings::default();
    };
    for (k, v) in stored {
        if !merged.contains_key(&k) {
            continue;
        }
        let previous = merged.insert(k.clone(), v);
        if !parses(&merged) {
            if let Some(p) = previous {
                merged.insert(k, p);
            }
        }
    }
    serde_json::from_value(Value::Object(merged)).unwrap_or_default()
}

fn parses(m: &Map<String, Value>) -> bool {
    serde_json::from_value::<AppSettings>(Value::Object(m.clone())).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use fazasanj_model::{AiProvider, Language, ThemePref};

    #[test]
    fn kv_roundtrip() {
        let s = Store::open_in_memory().unwrap();
        assert!(s.get_value("ai.model.open_ai").unwrap().is_none());
        s.set_value("ai.model.open_ai", "gpt-5-mini").unwrap();
        s.set_value("ai.model.open_ai", "gpt-5").unwrap();
        assert_eq!(s.get_value("ai.model.open_ai").unwrap().as_deref(), Some("gpt-5"));
        s.delete_value("ai.model.open_ai").unwrap();
        assert!(s.get_value("ai.model.open_ai").unwrap().is_none());
    }

    #[test]
    fn settings_default_when_missing() {
        let s = Store::open_in_memory().unwrap();
        assert_eq!(s.get_settings().unwrap(), AppSettings::default());
    }

    #[test]
    fn settings_roundtrip() {
        let s = Store::open_in_memory().unwrap();
        let st = AppSettings {
            language: Language::En,
            ai_default_provider: Some(AiProvider::Groq),
            excluded_paths: vec!["D:\\VMs".into()],
            ..AppSettings::default()
        };
        s.set_settings(&st).unwrap();
        assert_eq!(s.get_settings().unwrap(), st);
    }

    #[test]
    fn settings_tolerate_bad_fields() {
        let raw = r#"{"language":"en","theme":"neon","staleMonths":"x","futureThing":1,"trayEnabled":true}"#;
        let st = settings_from_json(raw);
        assert_eq!(st.language, Language::En);
        assert_eq!(st.theme, ThemePref::System);
        assert_eq!(st.stale_months, 12);
        assert!(st.tray_enabled);
    }

    #[test]
    fn settings_tolerate_garbage() {
        let s = Store::open_in_memory().unwrap();
        s.set_value(SETTINGS_KEY, "not json").unwrap();
        assert_eq!(s.get_settings().unwrap(), AppSettings::default());
        s.set_value(SETTINGS_KEY, "[1,2]").unwrap();
        assert_eq!(s.get_settings().unwrap(), AppSettings::default());
    }
}
