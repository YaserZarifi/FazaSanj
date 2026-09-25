//! Turns the model's text into a checked answer. Anything off (not JSON, missing fields,
//! unknown safety value) is an `invalid_response`; the UI then says "could not analyze".

use fazasanj_model::SafetyLevel;
use serde_json::{Map, Value};

use crate::AiError;

/// Longest value we keep, in characters. Longer text is cut, not rejected.
pub const MAX_FIELD_CHARS: usize = 600;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedAnswer {
    pub what: String,
    pub why_big: String,
    /// Never `Safe`.
    pub safety: SafetyLevel,
    pub consequence: String,
}

pub fn parse_answer(text: &str) -> Result<CheckedAnswer, AiError> {
    let json = extract_json(text).ok_or_else(|| invalid("no json object"))?;
    let value: Value = serde_json::from_str(json).map_err(|_| invalid("not valid json"))?;
    let obj = value.as_object().ok_or_else(|| invalid("not an object"))?;
    Ok(CheckedAnswer {
        what: field(obj, "what")?,
        why_big: field(obj, "why_big")?,
        safety: safety(obj)?,
        consequence: field(obj, "consequence")?,
    })
}

fn invalid(why: &str) -> AiError {
    AiError::InvalidResponse(why.to_owned())
}

/// Accepts plain JSON, JSON inside ``` fences, or JSON with a bit of chatter around it.
fn extract_json(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    (end > start).then(|| &text[start..=end])
}

fn field(obj: &Map<String, Value>, name: &str) -> Result<String, AiError> {
    let raw = obj
        .get(name)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AiError::InvalidResponse(format!("missing {name}")))?;
    Ok(cap(raw))
}

fn cap(s: &str) -> String {
    if s.chars().count() <= MAX_FIELD_CHARS {
        return s.to_owned();
    }
    let mut out: String = s.chars().take(MAX_FIELD_CHARS - 1).collect();
    out.push('…');
    out
}

fn safety(obj: &Map<String, Value>) -> Result<SafetyLevel, AiError> {
    let raw = obj
        .get("safety")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("missing safety"))?;
    let norm = raw.trim().to_lowercase().replace([' ', '-'], "_");
    let level = match norm.as_str() {
        "safe" | "probably_safe" => SafetyLevel::ProbablySafe,
        "careful" => SafetyLevel::Careful,
        "do_not_touch" => SafetyLevel::DoNotTouch,
        _ => return Err(invalid("unknown safety value")),
    };
    Ok(level.at_most(SafetyLevel::ProbablySafe))
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"{"what":"Telegram cache","why_big":"Saved media","safety":"probably_safe","consequence":"Media downloads again"}"#;

    #[test]
    fn valid_answer() {
        let a = parse_answer(GOOD).unwrap();
        assert_eq!(a.what, "Telegram cache");
        assert_eq!(a.safety, SafetyLevel::ProbablySafe);
    }

    #[test]
    fn fenced_and_chatty() {
        let fenced = format!("```json\n{GOOD}\n```");
        assert!(parse_answer(&fenced).is_ok());
        let chatty = format!("Sure! Here you go:\n{GOOD}\nHope this helps.");
        assert!(parse_answer(&chatty).is_ok());
    }

    #[test]
    fn safe_is_capped() {
        let a = parse_answer(&GOOD.replace("probably_safe", "safe")).unwrap();
        assert_eq!(a.safety, SafetyLevel::ProbablySafe);
        let a = parse_answer(&GOOD.replace("probably_safe", "Do Not Touch")).unwrap();
        assert_eq!(a.safety, SafetyLevel::DoNotTouch);
    }

    #[test]
    fn persian_values_pass() {
        let fa = r#"{"what":"حافظهٔ موقت تلگرام","why_big":"عکس‌ها و ویدیوهای ذخیره‌شده","safety":"careful","consequence":"فایل‌ها دوباره دانلود می‌شوند"}"#;
        assert_eq!(parse_answer(fa).unwrap().safety, SafetyLevel::Careful);
    }

    #[test]
    fn rejects_bad_answers() {
        let cases = [
            "",
            "I cannot help with that.",
            "{not json}",
            "[1,2,3]",
            r#"{"what":"x","why_big":"y","consequence":"z"}"#,
            r#"{"what":"","why_big":"y","safety":"careful","consequence":"z"}"#,
            r#"{"what":"x","why_big":"y","safety":"totally_fine","consequence":"z"}"#,
            r#"{"what":"x","why_big":5,"safety":"careful","consequence":"z"}"#,
        ];
        for c in cases {
            let e = parse_answer(c).unwrap_err();
            assert_eq!(e.code(), "invalid_response", "case {c:?}");
        }
    }

    #[test]
    fn long_values_are_cut() {
        let long = "a".repeat(5000);
        let a = parse_answer(&GOOD.replace("Telegram cache", &long)).unwrap();
        assert_eq!(a.what.chars().count(), MAX_FIELD_CHARS);
    }
}
