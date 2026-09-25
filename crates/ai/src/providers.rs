//! Request shapes and response parsing per provider. Kept free of HTTP so it can be tested
//! with recorded bodies.

use fazasanj_model::AiProvider;
use serde_json::{json, Value};

use crate::keys::ApiKey;
use crate::AiError;

/// Room for the answer plus any thinking the model does first. The answer itself is small.
const ANTHROPIC_MAX_TOKENS: u32 = 4096;
const ANTHROPIC_VERSION: &str = "2023-06-01";

pub fn default_base_url(provider: AiProvider) -> &'static str {
    match provider {
        AiProvider::OpenAi => "https://api.openai.com/v1",
        AiProvider::Gemini => "https://generativelanguage.googleapis.com/v1beta",
        AiProvider::Anthropic => "https://api.anthropic.com/v1",
        AiProvider::Groq => "https://api.groq.com/openai/v1",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Method {
    Get,
    Post,
}

/// One HTTP call. Custom Debug so the key header value never shows up in logs.
pub(crate) struct PreparedRequest {
    pub method: Method,
    pub url: String,
    pub headers: Vec<(&'static str, String)>,
    pub body: Option<Value>,
}

impl std::fmt::Debug for PreparedRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names: Vec<&str> = self.headers.iter().map(|(n, _)| *n).collect();
        f.debug_struct("PreparedRequest")
            .field("method", &self.method)
            .field("url", &self.url)
            .field("headers", &names)
            .finish_non_exhaustive()
    }
}

/// Model names go into Gemini URLs, so only plain characters are allowed.
pub(crate) fn check_model(model: &str) -> Result<&str, AiError> {
    let model = model.trim();
    let model = model.strip_prefix("models/").unwrap_or(model);
    let ok = !model.is_empty()
        && model.len() <= 100
        && !model.contains("..")
        && model
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-._:/".contains(c));
    if ok {
        Ok(model)
    } else {
        Err(AiError::ProviderError {
            status: None,
            message: "bad_model_name".to_owned(),
        })
    }
}

fn auth_headers(provider: AiProvider, key: &ApiKey) -> Vec<(&'static str, String)> {
    match provider {
        AiProvider::OpenAi | AiProvider::Groq => {
            vec![("authorization", format!("Bearer {}", key.expose()))]
        }
        // Header instead of ?key= so the key never sits in a URL.
        AiProvider::Gemini => vec![("x-goog-api-key", key.expose().to_owned())],
        AiProvider::Anthropic => vec![
            ("x-api-key", key.expose().to_owned()),
            ("anthropic-version", ANTHROPIC_VERSION.to_owned()),
        ],
    }
}

pub(crate) fn explain_request(
    provider: AiProvider,
    base_url: &str,
    model: &str,
    system: &str,
    user: &str,
    key: &ApiKey,
) -> Result<PreparedRequest, AiError> {
    let model = check_model(model)?;
    let base = base_url.trim_end_matches('/');
    let (url, body) = match provider {
        AiProvider::OpenAi | AiProvider::Groq => {
            let mut body = json!({
                "model": model,
                "messages": [
                    { "role": "system", "content": system },
                    { "role": "user", "content": user },
                ],
                "response_format": { "type": "json_object" },
            });
            // Reasoning models spend a long time thinking by default; this task is simple.
            if provider == AiProvider::OpenAi && model.starts_with("gpt-5") {
                body["reasoning_effort"] = json!("low");
            }
            (format!("{base}/chat/completions"), body)
        }
        AiProvider::Gemini => (
            format!("{base}/models/{model}:generateContent"),
            json!({
                "systemInstruction": { "parts": [{ "text": system }] },
                "contents": [{ "role": "user", "parts": [{ "text": user }] }],
                "generationConfig": { "responseMimeType": "application/json" },
            }),
        ),
        AiProvider::Anthropic => (
            format!("{base}/messages"),
            json!({
                "model": model,
                "max_tokens": ANTHROPIC_MAX_TOKENS,
                "system": system,
                "messages": [{ "role": "user", "content": user }],
            }),
        ),
    };
    Ok(PreparedRequest {
        method: Method::Post,
        url,
        headers: auth_headers(provider, key),
        body: Some(body),
    })
}

/// Listing models costs nothing and still fails with 401/403 on a bad key.
pub(crate) fn test_request(provider: AiProvider, base_url: &str, key: &ApiKey) -> PreparedRequest {
    let base = base_url.trim_end_matches('/');
    PreparedRequest {
        method: Method::Get,
        url: format!("{base}/models"),
        headers: auth_headers(provider, key),
        body: None,
    }
}

/// Pulls the model's text out of a successful response body.
pub(crate) fn extract_text(provider: AiProvider, body: &Value) -> Result<String, AiError> {
    let text = match provider {
        AiProvider::OpenAi | AiProvider::Groq => body
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str)
            .map(str::to_owned),
        AiProvider::Gemini => {
            if body.pointer("/promptFeedback/blockReason").is_some() {
                return Err(AiError::InvalidResponse("blocked".to_owned()));
            }
            body.pointer("/candidates/0/content/parts")
                .and_then(Value::as_array)
                .map(|parts| {
                    parts
                        .iter()
                        .filter(|p| p.get("thought").and_then(Value::as_bool) != Some(true))
                        .filter_map(|p| p.get("text").and_then(Value::as_str))
                        .collect::<String>()
                })
        }
        AiProvider::Anthropic => {
            if body.get("stop_reason").and_then(Value::as_str) == Some("refusal") {
                return Err(AiError::InvalidResponse("refused".to_owned()));
            }
            body.get("content").and_then(Value::as_array).map(|blocks| {
                blocks
                    .iter()
                    .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
                    .filter_map(|b| b.get("text").and_then(Value::as_str))
                    .collect::<String>()
            })
        }
    };
    text.filter(|t| !t.trim().is_empty())
        .ok_or_else(|| AiError::InvalidResponse("empty answer".to_owned()))
}

/// The provider's own error message, if the body has one. Every provider we support uses
/// `{"error": {"message": ...}}`.
pub(crate) fn error_message(body: &str) -> Option<String> {
    let v: Value = serde_json::from_str(body).ok()?;
    let msg = v.pointer("/error/message").and_then(Value::as_str)?;
    Some(msg.chars().take(300).collect())
}

/// Gemini answers a bad key with 400 instead of 401.
pub(crate) fn is_bad_key_body(provider: AiProvider, body: &str) -> bool {
    provider == AiProvider::Gemini
        && (body.contains("API_KEY_INVALID") || body.contains("API key not valid"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> ApiKey {
        ApiKey::new("sk-test-secret")
    }

    // Trimmed copies of real response bodies.
    const OPENAI_OK: &str = r#"{
      "id": "chatcmpl-1", "object": "chat.completion", "model": "gpt-5-mini",
      "choices": [{"index": 0, "finish_reason": "stop",
        "message": {"role": "assistant", "content": "{\"what\":\"a\",\"why_big\":\"b\",\"safety\":\"careful\",\"consequence\":\"c\"}", "refusal": null}}],
      "usage": {"prompt_tokens": 300, "completion_tokens": 40, "total_tokens": 340}
    }"#;

    const GROQ_OK: &str = r#"{
      "id": "chatcmpl-2", "object": "chat.completion", "model": "llama-3.3-70b-versatile",
      "choices": [{"index": 0, "finish_reason": "stop",
        "message": {"role": "assistant", "content": "```json\n{\"what\":\"a\",\"why_big\":\"b\",\"safety\":\"safe\",\"consequence\":\"c\"}\n```"}}],
      "x_groq": {"id": "req_1"}
    }"#;

    const GEMINI_OK: &str = r#"{
      "candidates": [{"content": {"role": "model", "parts": [
          {"text": "thinking...", "thought": true},
          {"text": "{\"what\":\"a\",\"why_big\":\"b\","},
          {"text": "\"safety\":\"do_not_touch\",\"consequence\":\"c\"}"}]},
        "finishReason": "STOP", "index": 0}],
      "usageMetadata": {"promptTokenCount": 300, "candidatesTokenCount": 40},
      "modelVersion": "gemini-2.5-flash"
    }"#;

    const GEMINI_BLOCKED: &str = r#"{"promptFeedback": {"blockReason": "SAFETY"}}"#;

    const ANTHROPIC_OK: &str = r#"{
      "id": "msg_1", "type": "message", "role": "assistant", "model": "claude-sonnet-5",
      "content": [
        {"type": "thinking", "thinking": "", "signature": "abc"},
        {"type": "text", "text": "{\"what\":\"a\",\"why_big\":\"b\",\"safety\":\"probably_safe\",\"consequence\":\"c\"}"}],
      "stop_reason": "end_turn", "usage": {"input_tokens": 300, "output_tokens": 60}
    }"#;

    const ANTHROPIC_REFUSAL: &str = r#"{
      "id": "msg_2", "type": "message", "role": "assistant", "content": [],
      "stop_reason": "refusal", "stop_details": {"type": "refusal", "category": null}
    }"#;

    fn parse(provider: AiProvider, body: &str) -> Result<String, AiError> {
        extract_text(provider, &serde_json::from_str(body).unwrap())
    }

    #[test]
    fn parses_each_provider() {
        use crate::validate::parse_answer;
        use fazasanj_model::SafetyLevel;
        let openai = parse_answer(&parse(AiProvider::OpenAi, OPENAI_OK).unwrap()).unwrap();
        assert_eq!(openai.safety, SafetyLevel::Careful);
        let groq = parse_answer(&parse(AiProvider::Groq, GROQ_OK).unwrap()).unwrap();
        assert_eq!(groq.safety, SafetyLevel::ProbablySafe);
        let gemini = parse(AiProvider::Gemini, GEMINI_OK).unwrap();
        assert!(!gemini.contains("thinking"));
        assert_eq!(
            parse_answer(&gemini).unwrap().safety,
            SafetyLevel::DoNotTouch
        );
        let claude = parse_answer(&parse(AiProvider::Anthropic, ANTHROPIC_OK).unwrap()).unwrap();
        assert_eq!(claude.what, "a");
    }

    #[test]
    fn blocked_and_empty_are_invalid() {
        for (p, body) in [
            (AiProvider::Gemini, GEMINI_BLOCKED),
            (AiProvider::Anthropic, ANTHROPIC_REFUSAL),
            (
                AiProvider::OpenAi,
                r#"{"choices":[{"message":{"content":""}}]}"#,
            ),
            (AiProvider::Groq, r#"{"unexpected":true}"#),
        ] {
            assert_eq!(parse(p, body).unwrap_err().code(), "invalid_response");
        }
    }

    #[test]
    fn request_shapes() {
        let r = explain_request(
            AiProvider::Anthropic,
            "https://x/v1/",
            "claude-sonnet-5",
            "S",
            "U",
            &key(),
        )
        .unwrap();
        assert_eq!(r.url, "https://x/v1/messages");
        assert!(r
            .headers
            .iter()
            .any(|(n, v)| *n == "x-api-key" && v == "sk-test-secret"));
        assert!(r
            .headers
            .iter()
            .any(|(n, v)| *n == "anthropic-version" && v == "2023-06-01"));
        assert_eq!(r.body.as_ref().unwrap()["system"], "S");

        let r = explain_request(
            AiProvider::Gemini,
            "https://g/v1beta",
            "models/gemini-2.5-flash",
            "S",
            "U",
            &key(),
        )
        .unwrap();
        assert_eq!(
            r.url,
            "https://g/v1beta/models/gemini-2.5-flash:generateContent"
        );
        assert!(!r.url.contains("secret"));
        let b = r.body.unwrap();
        assert_eq!(
            b["generationConfig"]["responseMimeType"],
            "application/json"
        );

        let r = explain_request(
            AiProvider::OpenAi,
            "https://o/v1",
            "gpt-5-mini",
            "S",
            "U",
            &key(),
        )
        .unwrap();
        let b = r.body.unwrap();
        assert_eq!(b["response_format"]["type"], "json_object");
        assert_eq!(b["reasoning_effort"], "low");
        assert!(r
            .headers
            .iter()
            .any(|(n, v)| *n == "authorization" && v == "Bearer sk-test-secret"));

        let r = explain_request(
            AiProvider::Groq,
            "https://q/openai/v1",
            "llama-3.3-70b-versatile",
            "S",
            "U",
            &key(),
        )
        .unwrap();
        assert!(r.body.unwrap().get("reasoning_effort").is_none());
    }

    #[test]
    fn body_never_has_key_and_debug_hides_it() {
        for p in crate::models::ALL_PROVIDERS {
            let r = explain_request(p, default_base_url(p), "m-1", "S", "U", &key()).unwrap();
            assert!(!r.body.as_ref().unwrap().to_string().contains("secret"));
            assert!(!r.url.contains("secret"));
            assert!(!format!("{r:?}").contains("secret"));
        }
    }

    #[test]
    fn bad_model_names_rejected() {
        assert!(check_model("gemini-2.5-flash").is_ok());
        assert!(check_model("openai/gpt-oss-120b").is_ok());
        for bad in ["", "a b", "x?key=1", "../../x", "m#frag"] {
            assert!(check_model(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn error_bodies() {
        let body = r#"{"error":{"message":"Rate limit reached","type":"tokens"}}"#;
        assert_eq!(error_message(body).as_deref(), Some("Rate limit reached"));
        assert!(error_message("<html>").is_none());
        let gem = r#"{"error":{"code":400,"message":"API key not valid.","status":"INVALID_ARGUMENT","details":[{"reason":"API_KEY_INVALID"}]}}"#;
        assert!(is_bad_key_body(AiProvider::Gemini, gem));
        assert!(!is_bad_key_body(AiProvider::OpenAi, gem));
    }
}
