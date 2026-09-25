//! Cache of AI answers (P9). The key is a hash built by the ai crate from provider, model,
//! language and payload, so a changed folder gets a fresh answer.

use fazasanj_model::AiAnswer;
use rusqlite::{params, OptionalExtension};

use crate::{Result, Store};

impl Store {
    /// Returns the cached answer with `cached` set to true. A row that no longer parses
    /// (older app version) counts as a miss.
    pub fn ai_cache_get(&self, key: &str) -> Result<Option<AiAnswer>> {
        let raw: Option<String> = self
            .conn()
            .query_row("SELECT answer FROM ai_cache WHERE key = ?1", [key], |r| r.get(0))
            .optional()?;
        Ok(raw
            .and_then(|r| serde_json::from_str::<AiAnswer>(&r).ok())
            .map(|mut a| {
                a.cached = true;
                a
            }))
    }

    pub fn ai_cache_put(&self, key: &str, answer: &AiAnswer) -> Result<()> {
        let raw = serde_json::to_string(answer)?;
        self.conn().execute(
            "INSERT INTO ai_cache (key, answer, created_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET answer = excluded.answer,
                                            created_at = excluded.created_at",
            params![key, raw, answer.created_at],
        )?;
        Ok(())
    }

    pub fn ai_cache_clear(&self) -> Result<()> {
        self.conn().execute("DELETE FROM ai_cache", [])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fazasanj_model::{AiProvider, SafetyLevel};

    fn answer() -> AiAnswer {
        AiAnswer {
            path: "C:\\Users\\<user>\\AppData\\Local\\Foo".into(),
            what: "cache".into(),
            why_big: "downloads".into(),
            safety: SafetyLevel::Careful,
            consequence: "re-downloads".into(),
            language: "en".into(),
            provider: AiProvider::Gemini,
            model: "gemini-2.5-flash".into(),
            cached: false,
            created_at: 42,
        }
    }

    #[test]
    fn put_get_marks_cached() {
        let s = Store::open_in_memory().unwrap();
        assert!(s.ai_cache_get("k").unwrap().is_none());
        s.ai_cache_put("k", &answer()).unwrap();
        let got = s.ai_cache_get("k").unwrap().unwrap();
        assert!(got.cached);
        assert_eq!(got.what, "cache");
        let mut newer = answer();
        newer.what = "updated".into();
        s.ai_cache_put("k", &newer).unwrap();
        assert_eq!(s.ai_cache_get("k").unwrap().unwrap().what, "updated");
        s.ai_cache_clear().unwrap();
        assert!(s.ai_cache_get("k").unwrap().is_none());
    }

    #[test]
    fn corrupt_row_is_a_miss() {
        let s = Store::open_in_memory().unwrap();
        s.conn()
            .execute("INSERT INTO ai_cache (key, answer, created_at) VALUES ('k', '{bad', 1)", [])
            .unwrap();
        assert!(s.ai_cache_get("k").unwrap().is_none());
    }
}
