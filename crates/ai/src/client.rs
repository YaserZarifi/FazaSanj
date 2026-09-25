//! HTTP side: sends prepared requests and maps failures to [`AiError`].
//!
//! Needs a tokio runtime (Tauri's async commands provide one).

use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use fazasanj_model::{AiAnswer, AiProvider, Language};
use reqwest::header::RETRY_AFTER;
use reqwest::StatusCode;
use serde_json::Value;

use crate::error::redact;
use crate::keys::ApiKey;
use crate::prompt::{language_code, system_prompt, user_message};
use crate::providers::{self, Method, PreparedRequest};
use crate::validate::parse_answer;
use crate::AiError;

pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
pub const TOTAL_TIMEOUT: Duration = Duration::from_secs(45);

#[derive(Debug, Clone)]
pub struct AiClient {
    http: reqwest::Client,
    base_urls: HashMap<AiProvider, String>,
}

impl AiClient {
    pub fn new() -> Result<Self, AiError> {
        Self::with_timeouts(CONNECT_TIMEOUT, TOTAL_TIMEOUT)
    }

    pub fn with_timeouts(connect: Duration, total: Duration) -> Result<Self, AiError> {
        let http = reqwest::Client::builder()
            .connect_timeout(connect)
            .timeout(total)
            .user_agent(concat!("Fazasanj/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|_| AiError::ProviderError {
                status: None,
                message: "http_client_init_failed".to_owned(),
            })?;
        Ok(AiClient {
            http,
            base_urls: HashMap::new(),
        })
    }

    /// Points a provider at another base URL (tests, or a compatible proxy).
    pub fn set_base_url(&mut self, provider: AiProvider, url: impl Into<String>) {
        self.base_urls.insert(provider, url.into());
    }

    fn base_url(&self, provider: AiProvider) -> &str {
        self.base_urls
            .get(&provider)
            .map(String::as_str)
            .unwrap_or_else(|| providers::default_base_url(provider))
    }

    /// Asks the provider about one folder. `payload` must come from `build_payload`.
    /// The returned `path` is the (masked) payload path; the app should put the real path back.
    pub async fn explain(
        &self,
        provider: AiProvider,
        model: &str,
        key: &ApiKey,
        payload: &Value,
        language: Language,
    ) -> Result<AiAnswer, AiError> {
        if key.is_empty() {
            return Err(AiError::NoKey);
        }
        let req = providers::explain_request(
            provider,
            self.base_url(provider),
            model,
            &system_prompt(language),
            &user_message(payload),
            key,
        )?;
        let body = self.send(provider, req, key).await?;
        let json: Value = serde_json::from_str(&body)
            .map_err(|_| AiError::InvalidResponse("body is not json".to_owned()))?;
        let text = providers::extract_text(provider, &json)?;
        let checked = parse_answer(&text)?;
        Ok(AiAnswer {
            path: payload
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            what: checked.what,
            why_big: checked.why_big,
            safety: checked.safety,
            consequence: checked.consequence,
            language: language_code(language).to_owned(),
            provider,
            model: model.trim().to_owned(),
            cached: false,
            created_at: now_ms(),
        })
    }

    /// Checks the key with a free request (listing models). `model` is only checked for a
    /// valid shape, since model lists are paged and not worth walking.
    pub async fn test_key(
        &self,
        provider: AiProvider,
        model: &str,
        key: &ApiKey,
    ) -> Result<(), AiError> {
        if key.is_empty() {
            return Err(AiError::NoKey);
        }
        providers::check_model(model)?;
        let req = providers::test_request(provider, self.base_url(provider), key);
        self.send(provider, req, key).await.map(|_| ())
    }

    async fn send(
        &self,
        provider: AiProvider,
        req: PreparedRequest,
        key: &ApiKey,
    ) -> Result<String, AiError> {
        let mut builder = match req.method {
            Method::Get => self.http.get(&req.url),
            Method::Post => self.http.post(&req.url),
        };
        for (name, value) in &req.headers {
            builder = builder.header(*name, value);
        }
        if let Some(body) = &req.body {
            builder = builder
                .header("content-type", "application/json")
                .body(body.to_string());
        }
        let resp = builder.send().await.map_err(|e| map_transport(&e))?;
        let status = resp.status();
        let retry_after = resp
            .headers()
            .get(RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse::<u64>().ok());
        let body = resp.text().await.map_err(|e| map_transport(&e))?;
        if status.is_success() {
            return Ok(body);
        }
        Err(map_status(provider, status, retry_after, &body, key))
    }
}

/// reqwest error text can include the URL, so only the kind is used. A connection that never
/// gets established (DNS failure, refused, connect timeout) reads as "no internet" to a user.
fn map_transport(e: &reqwest::Error) -> AiError {
    if e.is_connect() {
        AiError::NoInternet
    } else if e.is_timeout() {
        AiError::Timeout
    } else if e.is_decode() || e.is_body() {
        AiError::InvalidResponse("could not read body".to_owned())
    } else {
        AiError::NoInternet
    }
}

fn map_status(
    provider: AiProvider,
    status: StatusCode,
    retry_after: Option<u64>,
    body: &str,
    key: &ApiKey,
) -> AiError {
    match status.as_u16() {
        401 | 403 => AiError::InvalidKey,
        400 if providers::is_bad_key_body(provider, body) => AiError::InvalidKey,
        429 => AiError::RateLimited {
            retry_after_secs: retry_after,
        },
        408 | 504 => AiError::Timeout,
        code => AiError::ProviderError {
            status: Some(code),
            message: providers::error_message(body)
                .map(|m| redact(&m, key.expose()))
                .unwrap_or_default(),
        },
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

fn shared() -> Result<&'static AiClient, AiError> {
    static CLIENT: OnceLock<Result<AiClient, AiError>> = OnceLock::new();
    CLIENT
        .get_or_init(AiClient::new)
        .as_ref()
        .map_err(Clone::clone)
}

/// [`AiClient::explain`] on a shared client with the default timeouts.
pub async fn explain(
    provider: AiProvider,
    model: &str,
    key: &ApiKey,
    payload: &Value,
    language: Language,
) -> Result<AiAnswer, AiError> {
    shared()?
        .explain(provider, model, key, payload, language)
        .await
}

/// [`AiClient::test_key`] on a shared client with the default timeouts.
pub async fn test_key(provider: AiProvider, model: &str, key: &ApiKey) -> Result<(), AiError> {
    shared()?.test_key(provider, model, key).await
}
