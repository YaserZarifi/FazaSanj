use fazasanj_model::ApiError;
use thiserror::Error;

/// Errors from the AI layer. None of the variants carries the API key, and provider messages
/// are scrubbed of it before they get here (see `redact`).
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AiError {
    #[error("no internet connection")]
    NoInternet,
    #[error("the request timed out")]
    Timeout,
    #[error("the API key was rejected")]
    InvalidKey,
    #[error("rate limited by the provider")]
    RateLimited { retry_after_secs: Option<u64> },
    #[error("provider error (status {status:?}): {message}")]
    ProviderError {
        status: Option<u16>,
        message: String,
    },
    #[error("the answer could not be understood: {0}")]
    InvalidResponse(String),
    #[error("no API key saved for this provider")]
    NoKey,
    #[error("could not use the Windows credential store: {0}")]
    KeyStore(String),
}

impl AiError {
    /// Stable code for the UI (`errors.<code>`).
    pub fn code(&self) -> &'static str {
        match self {
            AiError::NoInternet => "no_internet",
            AiError::Timeout => "timeout",
            AiError::InvalidKey => "invalid_key",
            AiError::RateLimited { .. } => "rate_limited",
            AiError::ProviderError { .. } => "provider_error",
            AiError::InvalidResponse(_) => "invalid_response",
            AiError::NoKey => "no_key",
            AiError::KeyStore(_) => "key_store_error",
        }
    }

    pub fn to_api_error(&self) -> ApiError {
        match self {
            AiError::RateLimited {
                retry_after_secs: Some(s),
            } => ApiError::with_detail(self.code(), s.to_string()),
            AiError::ProviderError { message, .. } if !message.is_empty() => {
                ApiError::with_detail(self.code(), message.clone())
            }
            AiError::InvalidResponse(m) | AiError::KeyStore(m) => {
                ApiError::with_detail(self.code(), m.clone())
            }
            _ => ApiError::new(self.code()),
        }
    }
}

impl From<AiError> for ApiError {
    fn from(e: AiError) -> Self {
        e.to_api_error()
    }
}

/// Removes the key from text that came back from a provider. Some providers echo part or all
/// of a bad key in their error message.
pub(crate) fn redact(text: &str, key: &str) -> String {
    let key = key.trim();
    if key.len() < 4 {
        return text.to_owned();
    }
    let mut out = text.replace(key, "[redacted]");
    // A long tail of the key is still a secret worth hiding.
    if key.len() >= 16 {
        if let Some(tail) = key.get(key.len() - 8..) {
            out = out.replace(tail, "[redacted]");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_stable() {
        assert_eq!(AiError::InvalidKey.code(), "invalid_key");
        assert_eq!(
            AiError::RateLimited {
                retry_after_secs: None
            }
            .code(),
            "rate_limited"
        );
        let api = AiError::RateLimited {
            retry_after_secs: Some(30),
        }
        .to_api_error();
        assert_eq!(api.detail.as_deref(), Some("30"));
    }

    #[test]
    fn redact_hides_key_and_tail() {
        let key = "sk-test-0123456789abcdefXYZW";
        let msg = format!("Incorrect API key provided: {key}. Also sk-****cdefXYZW");
        let r = redact(&msg, key);
        assert!(!r.contains(key));
        assert!(!r.contains("cdefXYZW"));
    }
}
