//! Model names offered in the UI. The first entry of each list is the default. The user can
//! also type any other model name, so this list only needs to be a good starting point.

use fazasanj_model::AiProvider;

const OPENAI: &[&str] = &["gpt-5-mini", "gpt-5", "gpt-5-nano", "gpt-4.1-mini"];
const GEMINI: &[&str] = &[
    "gemini-2.5-flash",
    "gemini-2.5-pro",
    "gemini-2.5-flash-lite",
];
const ANTHROPIC: &[&str] = &[
    "claude-sonnet-5",
    "claude-opus-5-5",
    "claude-haiku-4-5-20251001",
];
const GROQ: &[&str] = &[
    "llama-3.3-70b-versatile",
    "llama-3.1-8b-instant",
    "openai/gpt-oss-120b",
];

pub const ALL_PROVIDERS: [AiProvider; 4] = [
    AiProvider::OpenAi,
    AiProvider::Gemini,
    AiProvider::Anthropic,
    AiProvider::Groq,
];

pub fn available_models(provider: AiProvider) -> &'static [&'static str] {
    match provider {
        AiProvider::OpenAi => OPENAI,
        AiProvider::Gemini => GEMINI,
        AiProvider::Anthropic => ANTHROPIC,
        AiProvider::Groq => GROQ,
    }
}

pub fn default_model(provider: AiProvider) -> &'static str {
    available_models(provider)
        .first()
        .copied()
        .unwrap_or_default()
}

/// Stable id, same as the serde name. Used as the keyring account and in store keys.
pub fn provider_id(provider: AiProvider) -> &'static str {
    match provider {
        AiProvider::OpenAi => "open_ai",
        AiProvider::Gemini => "gemini",
        AiProvider::Anthropic => "anthropic",
        AiProvider::Groq => "groq",
    }
}

/// Human name for logs and the UI.
pub fn provider_name(provider: AiProvider) -> &'static str {
    match provider {
        AiProvider::OpenAi => "OpenAI",
        AiProvider::Gemini => "Google Gemini",
        AiProvider::Anthropic => "Anthropic",
        AiProvider::Groq => "Groq",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_provider_has_a_default() {
        for p in ALL_PROVIDERS {
            assert!(!default_model(p).is_empty());
            let serde_name = serde_json::to_value(p).unwrap();
            assert_eq!(serde_name.as_str(), Some(provider_id(p)));
        }
        assert_eq!(default_model(AiProvider::Anthropic), "claude-sonnet-5");
        assert_eq!(default_model(AiProvider::OpenAi), "gpt-5-mini");
    }
}
