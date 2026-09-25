//! Error mapping and request shape against a tiny local HTTP server. No real network.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use fazasanj_ai::{build_payload, AiClient, AiError, ApiKey, FolderMeta};
use fazasanj_model::{AiProvider, Language, SafetyLevel};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const KEY: &str = "sk-live-TOPSECRET-0123456789abcdef";

#[derive(Clone)]
struct Canned {
    status: u16,
    headers: Vec<(&'static str, String)>,
    body: String,
    delay: Duration,
}

impl Canned {
    fn json(status: u16, body: &str) -> Self {
        Canned {
            status,
            headers: vec![],
            body: body.to_owned(),
            delay: Duration::ZERO,
        }
    }
}

/// Serves `reply` to every connection and records the raw requests.
async fn serve(reply: Canned) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let seen2 = seen.clone();
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                return;
            };
            let reply = reply.clone();
            let seen = seen2.clone();
            tokio::spawn(async move {
                let raw = read_request(&mut sock).await;
                seen.lock().unwrap().push(raw);
                tokio::time::sleep(reply.delay).await;
                let mut head = format!(
                    "HTTP/1.1 {} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n",
                    reply.status,
                    reply.body.len()
                );
                for (k, v) in &reply.headers {
                    head.push_str(&format!("{k}: {v}\r\n"));
                }
                head.push_str("\r\n");
                let _ = sock.write_all(head.as_bytes()).await;
                let _ = sock.write_all(reply.body.as_bytes()).await;
                let _ = sock.shutdown().await;
            });
        }
    });
    (format!("http://{addr}/v1"), seen)
}

async fn read_request(sock: &mut tokio::net::TcpStream) -> String {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = sock.read(&mut chunk).await.unwrap_or(0);
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        let text = String::from_utf8_lossy(&buf).to_string();
        if let Some(end) = text.find("\r\n\r\n") {
            let len = text[..end]
                .lines()
                .find_map(|l| {
                    let (k, v) = l.split_once(':')?;
                    k.eq_ignore_ascii_case("content-length")
                        .then(|| v.trim().parse().ok())?
                })
                .unwrap_or(0usize);
            if buf.len() >= end + 4 + len {
                break;
            }
        }
    }
    String::from_utf8_lossy(&buf).to_string()
}

fn client(provider: AiProvider, base: &str) -> AiClient {
    let mut c =
        AiClient::with_timeouts(Duration::from_secs(2), Duration::from_millis(800)).unwrap();
    c.set_base_url(provider, base);
    c
}

fn payload() -> serde_json::Value {
    let meta = FolderMeta {
        path: r"C:\Users\Yaser\AppData\Local\Thing".into(),
        total_bytes: 1 << 30,
        file_count: 10,
        dir_count: 2,
        child_names: vec!["a".into()],
        ..FolderMeta::default()
    };
    build_payload(&meta, false, "Yaser")
}

fn key() -> ApiKey {
    ApiKey::new(KEY)
}

/// Every error surface we might log or show.
fn assert_no_key(e: &AiError) {
    for text in [
        format!("{e}"),
        format!("{e:?}"),
        format!("{:?}", e.to_api_error()),
    ] {
        assert!(!text.contains(KEY), "{text}");
        assert!(!text.contains("TOPSECRET"), "{text}");
    }
}

#[tokio::test]
async fn explain_openai_success() {
    let body = r#"{"choices":[{"message":{"content":"{\"what\":\"App cache\",\"why_big\":\"Downloads\",\"safety\":\"safe\",\"consequence\":\"Downloads again\"}"}}]}"#;
    let (base, seen) = serve(Canned::json(200, body)).await;
    let a = client(AiProvider::OpenAi, &base)
        .explain(
            AiProvider::OpenAi,
            "gpt-5-mini",
            &key(),
            &payload(),
            Language::En,
        )
        .await
        .unwrap();
    assert_eq!(a.what, "App cache");
    assert_eq!(a.safety, SafetyLevel::ProbablySafe);
    assert_eq!(a.language, "en");
    assert_eq!(a.path, r"C:\Users\<user>\AppData\Local\Thing");
    assert!(!a.cached);

    let raw = seen.lock().unwrap()[0].clone();
    assert!(raw.starts_with("POST /v1/chat/completions"));
    assert!(raw
        .to_lowercase()
        .contains(&format!("authorization: bearer {}", KEY.to_lowercase())));
    let body_part = &raw[raw.find("\r\n\r\n").unwrap()..];
    assert!(!body_part.contains(KEY), "key leaked into body");
    assert!(!body_part.contains("Yaser"), "username leaked into body");
}

#[tokio::test]
async fn explain_anthropic_headers_and_fa() {
    let body = r#"{"content":[{"type":"text","text":"{\"what\":\"کش برنامه\",\"why_big\":\"فایل‌های موقت\",\"safety\":\"careful\",\"consequence\":\"دوباره ساخته می‌شود\"}"}],"stop_reason":"end_turn"}"#;
    let (base, seen) = serve(Canned::json(200, body)).await;
    let a = client(AiProvider::Anthropic, &base)
        .explain(
            AiProvider::Anthropic,
            "claude-sonnet-5",
            &key(),
            &payload(),
            Language::Fa,
        )
        .await
        .unwrap();
    assert_eq!(a.language, "fa");
    assert_eq!(a.safety, SafetyLevel::Careful);
    let raw = seen.lock().unwrap()[0].to_lowercase();
    assert!(raw.starts_with("post /v1/messages"));
    assert!(raw.contains("x-api-key: "));
    assert!(raw.contains("anthropic-version: 2023-06-01"));
}

#[tokio::test]
async fn gemini_uses_header_not_url() {
    let body = r#"{"candidates":[{"content":{"parts":[{"text":"{\"what\":\"a\",\"why_big\":\"b\",\"safety\":\"do_not_touch\",\"consequence\":\"c\"}"}]}}]}"#;
    let (base, seen) = serve(Canned::json(200, body)).await;
    client(AiProvider::Gemini, &base)
        .explain(
            AiProvider::Gemini,
            "gemini-2.5-flash",
            &key(),
            &payload(),
            Language::En,
        )
        .await
        .unwrap();
    let raw = seen.lock().unwrap()[0].clone();
    let first_line = raw.lines().next().unwrap();
    assert_eq!(
        first_line,
        "POST /v1/models/gemini-2.5-flash:generateContent HTTP/1.1"
    );
    assert!(raw.to_lowercase().contains("x-goog-api-key: "));
}

#[tokio::test]
async fn unauthorized_maps_to_invalid_key() {
    let echo = format!(r#"{{"error":{{"message":"Incorrect API key provided: {KEY}"}}}}"#);
    for status in [401, 403] {
        let (base, _) = serve(Canned::json(status, &echo)).await;
        let e = client(AiProvider::OpenAi, &base)
            .explain(
                AiProvider::OpenAi,
                "gpt-5-mini",
                &key(),
                &payload(),
                Language::En,
            )
            .await
            .unwrap_err();
        assert_eq!(e, AiError::InvalidKey);
        assert_no_key(&e);
    }
}

#[tokio::test]
async fn gemini_bad_key_400_is_invalid_key() {
    let body = r#"{"error":{"code":400,"message":"API key not valid. Please pass a valid API key.","status":"INVALID_ARGUMENT"}}"#;
    let (base, _) = serve(Canned::json(400, body)).await;
    let e = client(AiProvider::Gemini, &base)
        .test_key(AiProvider::Gemini, "gemini-2.5-flash", &key())
        .await
        .unwrap_err();
    assert_eq!(e.code(), "invalid_key");
}

#[tokio::test]
async fn rate_limit_carries_retry_after() {
    let mut reply = Canned::json(429, r#"{"error":{"message":"slow down"}}"#);
    reply.headers.push(("retry-after", "17".into()));
    let (base, _) = serve(reply).await;
    let e = client(AiProvider::Groq, &base)
        .explain(
            AiProvider::Groq,
            "llama-3.3-70b-versatile",
            &key(),
            &payload(),
            Language::En,
        )
        .await
        .unwrap_err();
    assert_eq!(
        e,
        AiError::RateLimited {
            retry_after_secs: Some(17)
        }
    );
    assert_eq!(e.to_api_error().detail.as_deref(), Some("17"));
}

#[tokio::test]
async fn slow_server_times_out() {
    let mut reply = Canned::json(200, "{}");
    reply.delay = Duration::from_secs(3);
    let (base, _) = serve(reply).await;
    let e = client(AiProvider::OpenAi, &base)
        .explain(
            AiProvider::OpenAi,
            "gpt-5-mini",
            &key(),
            &payload(),
            Language::En,
        )
        .await
        .unwrap_err();
    assert_eq!(e.code(), "timeout");
}

#[tokio::test]
async fn closed_port_is_no_internet() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    // Windows retries a refused localhost connect for about 2s, so give it room.
    let mut c = AiClient::with_timeouts(Duration::from_secs(8), Duration::from_secs(10)).unwrap();
    c.set_base_url(AiProvider::OpenAi, format!("http://{addr}/v1"));
    let e = c
        .test_key(AiProvider::OpenAi, "gpt-5-mini", &key())
        .await
        .unwrap_err();
    assert_eq!(e.code(), "no_internet");
    assert_no_key(&e);
}

#[tokio::test]
async fn provider_error_is_redacted() {
    let echo = format!(r#"{{"error":{{"message":"internal failure for key {KEY}"}}}}"#);
    let (base, _) = serve(Canned::json(500, &echo)).await;
    let e = client(AiProvider::Anthropic, &base)
        .explain(
            AiProvider::Anthropic,
            "claude-sonnet-5",
            &key(),
            &payload(),
            Language::En,
        )
        .await
        .unwrap_err();
    assert_eq!(e.code(), "provider_error");
    assert!(format!("{e}").contains("internal failure"));
    assert_no_key(&e);
}

#[tokio::test]
async fn garbage_answer_is_invalid_response() {
    for body in [
        "not json at all",
        r#"{"choices":[{"message":{"content":"Sorry, I can't help."}}]}"#,
        r#"{"choices":[{"message":{"content":"{\"what\":\"x\"}"}}]}"#,
    ] {
        let (base, _) = serve(Canned::json(200, body)).await;
        let e = client(AiProvider::OpenAi, &base)
            .explain(
                AiProvider::OpenAi,
                "gpt-5-mini",
                &key(),
                &payload(),
                Language::En,
            )
            .await
            .unwrap_err();
        assert_eq!(e.code(), "invalid_response", "{body}");
    }
}

#[tokio::test]
async fn test_key_lists_models() {
    let (base, seen) = serve(Canned::json(200, r#"{"data":[]}"#)).await;
    client(AiProvider::Anthropic, &base)
        .test_key(AiProvider::Anthropic, "claude-sonnet-5", &key())
        .await
        .unwrap();
    assert!(seen.lock().unwrap()[0].starts_with("GET /v1/models "));
}

#[tokio::test]
async fn empty_key_is_rejected_before_network() {
    let c = AiClient::new().unwrap();
    let e = c
        .test_key(AiProvider::OpenAi, "gpt-5-mini", &ApiKey::new("   "))
        .await
        .unwrap_err();
    assert_eq!(e, AiError::NoKey);
}
