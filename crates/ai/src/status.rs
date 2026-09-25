use fazasanj_model::{AiProvider, AiProviderStatus};

use crate::models::{available_models, default_model};

/// Status row for the settings screen. `chosen_model` is what the user picked earlier
/// (stored by the app); an empty or missing value falls back to the default.
pub fn provider_status(provider: AiProvider, chosen_model: Option<&str>) -> AiProviderStatus {
    status_with(provider, chosen_model, crate::keys::has(provider))
}

fn status_with(provider: AiProvider, chosen: Option<&str>, has_key: bool) -> AiProviderStatus {
    let model = chosen
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .unwrap_or_else(|| default_model(provider));
    AiProviderStatus {
        provider,
        has_key,
        model: model.to_owned(),
        available_models: available_models(provider)
            .iter()
            .map(|m| (*m).to_owned())
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn falls_back_to_default_model() {
        let s = status_with(AiProvider::Gemini, Some("  "), false);
        assert_eq!(s.model, "gemini-2.5-flash");
        assert!(!s.has_key);
        let s = status_with(AiProvider::Gemini, Some("gemini-3-pro"), true);
        assert_eq!(s.model, "gemini-3-pro");
        assert_eq!(s.available_models[0], "gemini-2.5-flash");
    }
}
