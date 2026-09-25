//! Cache key for an answer: blake3 over provider, model, language and the canonical payload.
//! Object keys are sorted by hand so the key does not depend on serde_json's map order
//! (another crate in the app could turn on `preserve_order`).

use fazasanj_model::{AiProvider, Language};
use serde_json::Value;

use crate::models::provider_id;
use crate::prompt::language_code;

/// Bump when the prompt changes enough that old answers should not be reused.
const PROMPT_VERSION: u32 = 1;

pub fn cache_key(provider: AiProvider, model: &str, payload: &Value, language: Language) -> String {
    let mut canon = String::new();
    write_canonical(payload, &mut canon);
    let mut h = blake3::Hasher::new();
    for part in [
        &format!("v{PROMPT_VERSION}"),
        provider_id(provider),
        model.trim(),
        language_code(language),
        &canon,
    ] {
        h.update(part.as_bytes());
        h.update(&[0]);
    }
    h.finalize().to_hex().to_string()
}

fn write_canonical(v: &Value, out: &mut String) {
    match v {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push('{');
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&Value::String((*k).clone()).to_string());
                out.push(':');
                if let Some(child) = map.get(*k) {
                    write_canonical(child, out);
                }
            }
            out.push('}');
        }
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(item, out);
            }
            out.push(']');
        }
        other => out.push_str(&other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn stable_and_order_independent() {
        let a = json!({"path": "C:\\x", "n": 1, "kids": ["a", "b"]});
        let b: Value = serde_json::from_str(r#"{"kids":["a","b"],"n":1,"path":"C:\\x"}"#).unwrap();
        let ka = cache_key(AiProvider::Groq, "m", &a, Language::En);
        assert_eq!(ka, cache_key(AiProvider::Groq, "m", &b, Language::En));
        assert_eq!(ka.len(), 64);
    }

    #[test]
    fn differs_by_every_input() {
        let p = json!({"path": "C:\\x"});
        let base = cache_key(AiProvider::Groq, "m", &p, Language::En);
        assert_ne!(base, cache_key(AiProvider::OpenAi, "m", &p, Language::En));
        assert_ne!(base, cache_key(AiProvider::Groq, "m2", &p, Language::En));
        assert_ne!(base, cache_key(AiProvider::Groq, "m", &p, Language::Fa));
        assert_ne!(
            base,
            cache_key(
                AiProvider::Groq,
                "m",
                &json!({"path": "C:\\y"}),
                Language::En
            )
        );
    }
}
