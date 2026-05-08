mod jcs;
mod ledger;
mod ulid;

use anyhow::Result;
use axum::{
    Router,
    body::Body,
    extract::State,
    http::{HeaderMap, HeaderName, HeaderValue, Request, StatusCode},
    response::Response,
    routing::post,
};
use bytes::Bytes;
use ledger::SessionLedger;
use serde_json::Value;
use std::{path::PathBuf, str::FromStr, sync::Arc};
use tracing::{error, info};

const SESSION_HEADER: &str = "x-harness-session";
const UPSTREAM_HEADER: &str = "x-harness-upstream";

#[derive(Clone)]
struct AppState {
    harness_root: PathBuf,
    upstream_base: String,
    anthropic_base: String,
    client: reqwest::Client,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        )
        .init();

    let harness_root = PathBuf::from(
        std::env::var("HARNESS_ROOT").unwrap_or_else(|_| ".harness".to_string()),
    );
    let upstream_base = std::env::var("UPSTREAM_BASE_URL")
        .unwrap_or_else(|_| "https://api.openai.com".to_string())
        .trim_end_matches('/')
        .to_string();
    let anthropic_base = std::env::var("ANTHROPIC_BASE_URL")
        .unwrap_or_else(|_| "https://api.anthropic.com".to_string())
        .trim_end_matches('/')
        .to_string();

    let listen = std::env::var("HARNESS_LISTEN")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string());

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .connection_verbose(false)
        .build()?;

    let state = Arc::new(AppState {
        harness_root,
        upstream_base,
        anthropic_base,
        client,
    });

    let app = Router::new()
        .route("/v1/chat/completions", post(openai_handler))
        .route("/v1/messages", post(anthropic_handler))
        .with_state(state);

    info!("harness-proxy listening on {}", listen);
    let listener = tokio::net::TcpListener::bind(&listen).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

/// OpenAI-compatible handler €” intercept, ledger, release (fail-closed).
async fn openai_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, StatusCode> {
    let payload: Value = serde_json::from_slice(&body).map_err(|_| StatusCode::BAD_REQUEST)?;

    let sid = headers
        .get(SESSION_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
        .unwrap_or_else(ulid::new_ulid);

    let in_hash = ledger::hash_input(
        payload.get("system").and_then(|v| v.as_str()),
        payload.get("messages"),
        payload.get("tools"),
    );

    let target_base = headers
        .get(UPSTREAM_HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or(&state.upstream_base)
        .trim_end_matches('/');

    let upstream_url = format!("{}/v1/chat/completions", target_base);
    let model = payload
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let res_bytes = forward(&state.client, &upstream_url, &headers, body, &[SESSION_HEADER, UPSTREAM_HEADER])
        .await
        .map_err(|e| { error!("upstream error: {e}"); StatusCode::BAD_GATEWAY })?;

    let (reason, act) = extract_openai(&res_bytes);

    // FAIL-CLOSED: ledger must succeed before response is released.
    let entry = SessionLedger::append_entry(
        &state.harness_root,
        &sid,
        &model,
        &in_hash,
        &reason,
        act.as_ref(),
    )
    .map_err(|e| { error!("ledger write failed €” withholding response: {e}"); StatusCode::INTERNAL_SERVER_ERROR })?;

    let mut res = Response::new(Body::from(res_bytes));
    *res.status_mut() = StatusCode::OK;
    res.headers_mut().insert(
        HeaderName::from_str("x-harness-session").unwrap(),
        HeaderValue::from_str(&sid).unwrap(),
    );
    res.headers_mut().insert(
        HeaderName::from_str("x-harness-seq").unwrap(),
        HeaderValue::from_str(&entry.seq.to_string()).unwrap(),
    );
    res.headers_mut().insert(
        HeaderName::from_str("x-harness-prev").unwrap(),
        HeaderValue::from_str(&entry.prev).unwrap(),
    );
    Ok(res)
}

/// Anthropic-compatible handler.
async fn anthropic_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, StatusCode> {
    let payload: Value = serde_json::from_slice(&body).map_err(|_| StatusCode::BAD_REQUEST)?;

    let sid = headers
        .get(SESSION_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
        .unwrap_or_else(ulid::new_ulid);

    let system_str;
    let system = match payload.get("system") {
        Some(Value::String(s)) => Some(s.as_str()),
        Some(v) => { system_str = v.to_string(); Some(system_str.as_str()) }
        None => None,
    };

    let in_hash = ledger::hash_input(
        system,
        payload.get("messages"),
        payload.get("tools"),
    );

    let target_base = headers
        .get(UPSTREAM_HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or(&state.anthropic_base)
        .trim_end_matches('/');

    let upstream_url = format!("{}/v1/messages", target_base);
    let model = payload
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let res_bytes = forward(&state.client, &upstream_url, &headers, body, &[SESSION_HEADER, UPSTREAM_HEADER])
        .await
        .map_err(|e| { error!("upstream error: {e}"); StatusCode::BAD_GATEWAY })?;

    let (reason, act) = extract_anthropic(&res_bytes);

    let entry = SessionLedger::append_entry(
        &state.harness_root,
        &sid,
        &model,
        &in_hash,
        &reason,
        act.as_ref(),
    )
    .map_err(|e| { error!("ledger write failed €” withholding response: {e}"); StatusCode::INTERNAL_SERVER_ERROR })?;

    let mut res = Response::new(Body::from(res_bytes));
    *res.status_mut() = StatusCode::OK;
    res.headers_mut().insert(
        HeaderName::from_str("x-harness-session").unwrap(),
        HeaderValue::from_str(&sid).unwrap(),
    );
    res.headers_mut().insert(
        HeaderName::from_str("x-harness-seq").unwrap(),
        HeaderValue::from_str(&entry.seq.to_string()).unwrap(),
    );
    Ok(res)
}

/// Forward request to upstream, stripping harness-specific headers.
async fn forward(
    client: &reqwest::Client,
    url: &str,
    headers: &HeaderMap,
    body: Bytes,
    strip: &[&str],
) -> Result<Bytes> {
    let mut req = client.post(url).body(body);
    for (k, v) in headers.iter() {
        let name = k.as_str().to_lowercase();
        if name == "host" || name == "content-length" || strip.contains(&name.as_str()) {
            continue;
        }
        req = req.header(k.as_str(), v.as_bytes());
    }
    let res = req.send().await?;
    Ok(res.bytes().await?)
}

fn extract_openai(bytes: &[u8]) -> (String, Option<Value>) {
    let Ok(v) = serde_json::from_slice::<Value>(bytes) else { return (String::new(), None) };
    let reason = v["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let act = v["choices"][0]["message"]["tool_calls"].clone();
    let act = if act.is_null() { None } else { Some(act) };
    (reason, act)
}

fn extract_anthropic(bytes: &[u8]) -> (String, Option<Value>) {
    let Ok(v) = serde_json::from_slice::<Value>(bytes) else { return (String::new(), None) };
    let mut reason = String::new();
    let mut act = None;
    if let Some(content) = v["content"].as_array() {
        for block in content {
            match block["type"].as_str() {
                Some("text") => { reason = block["text"].as_str().unwrap_or("").to_string(); }
                Some("tool_use") => { act = Some(block.clone()); }
                _ => {}
            }
        }
    }
    (reason, act)
}
