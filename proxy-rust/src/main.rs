mod jcs;
mod ledger;
mod ulid;

use anyhow::Result;
use axum::{
    Router,
    body::Body,
    extract::State,
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::Response,
    routing::post,
};
use bytes::Bytes;
use futures_util::StreamExt as _;
use ledger::SessionLedger;
use serde_json::Value;
use std::{path::PathBuf, str::FromStr, sync::Arc};
use tokio_stream::wrappers::ReceiverStream;
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

    let upstream = send_upstream(&state.client, &upstream_url, &headers, body, &[SESSION_HEADER, UPSTREAM_HEADER])
        .await
        .map_err(|e| { error!("upstream error: {e}"); StatusCode::BAD_GATEWAY })?;

    let is_sse = upstream.headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("text/event-stream"))
        .unwrap_or(false);

    if is_sse {
        // Streaming path: tee chunks to client as they arrive, write ledger at stream end.
        // Fail-closed guarantee weakened for streaming: chunks already forwarded before ledger write.
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<Bytes, reqwest::Error>>(64);
        let root = state.harness_root.clone();
        let sid_task = sid.clone();
        let model_task = model.clone();
        let in_hash_task = in_hash.clone();
        tokio::spawn(async move {
            let mut stream = upstream.bytes_stream();
            let mut buf: Vec<u8> = Vec::new();
            while let Some(item) = stream.next().await {
                match item {
                    Ok(chunk) => {
                        buf.extend_from_slice(&chunk);
                        if tx.send(Ok(chunk)).await.is_err() { break; }
                    }
                    Err(e) => { let _ = tx.send(Err(e)).await; break; }
                }
            }
            let (reason, think, act) = accumulate_sse_openai(&buf);
            let has_think = think.is_some();
            let has_act = act.is_some();
            match SessionLedger::append_entry(&root, &sid_task, &model_task, &in_hash_task, has_think, think.as_ref(), has_act, act.as_ref(), &reason) {
                Ok(entry) => info!("stream ledger: sid={} seq={}", sid_task, entry.seq),
                Err(e) => error!("stream ledger write failed — stream unrecorded: {e}"),
            }
        });
        let mut res = Response::new(Body::from_stream(ReceiverStream::new(rx)));
        *res.status_mut() = StatusCode::OK;
        res.headers_mut().insert(
            HeaderName::from_str("x-harness-session").unwrap(),
            HeaderValue::from_str(&sid).unwrap(),
        );
        Ok(res)
    } else {
        // Buffered path: fail-closed guarantee intact.
        let res_bytes = upstream.bytes()
            .await
            .map_err(|e| { error!("upstream read error: {e}"); StatusCode::BAD_GATEWAY })?;
        let (reason, think, act) = extract_openai(&res_bytes);
        let entry = SessionLedger::append_entry(
            &state.harness_root, &sid, &model, &in_hash,
            think.is_some(), think.as_ref(), act.is_some(), act.as_ref(), &reason,
        )
        .map_err(|e| { error!("ledger write failed — withholding response: {e}"); StatusCode::INTERNAL_SERVER_ERROR })?;
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

    let upstream = send_upstream(&state.client, &upstream_url, &headers, body, &[SESSION_HEADER, UPSTREAM_HEADER])
        .await
        .map_err(|e| { error!("upstream error: {e}"); StatusCode::BAD_GATEWAY })?;

    let is_sse = upstream.headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("text/event-stream"))
        .unwrap_or(false);

    if is_sse {
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<Bytes, reqwest::Error>>(64);
        let root = state.harness_root.clone();
        let sid_task = sid.clone();
        let model_task = model.clone();
        let in_hash_task = in_hash.clone();
        tokio::spawn(async move {
            let mut stream = upstream.bytes_stream();
            let mut buf: Vec<u8> = Vec::new();
            while let Some(item) = stream.next().await {
                match item {
                    Ok(chunk) => {
                        buf.extend_from_slice(&chunk);
                        if tx.send(Ok(chunk)).await.is_err() { break; }
                    }
                    Err(e) => { let _ = tx.send(Err(e)).await; break; }
                }
            }
            let (reason, think, act) = accumulate_sse_anthropic(&buf);
            let has_think = think.is_some();
            let has_act = act.is_some();
            match SessionLedger::append_entry(&root, &sid_task, &model_task, &in_hash_task, has_think, think.as_ref(), has_act, act.as_ref(), &reason) {
                Ok(entry) => info!("stream ledger: sid={} seq={}", sid_task, entry.seq),
                Err(e) => error!("stream ledger write failed — stream unrecorded: {e}"),
            }
        });
        let mut res = Response::new(Body::from_stream(ReceiverStream::new(rx)));
        *res.status_mut() = StatusCode::OK;
        res.headers_mut().insert(
            HeaderName::from_str("x-harness-session").unwrap(),
            HeaderValue::from_str(&sid).unwrap(),
        );
        Ok(res)
    } else {
        let res_bytes = upstream.bytes()
            .await
            .map_err(|e| { error!("upstream read error: {e}"); StatusCode::BAD_GATEWAY })?;
        let (reason, think, act) = extract_anthropic(&res_bytes);
        let entry = SessionLedger::append_entry(
            &state.harness_root, &sid, &model, &in_hash,
            think.is_some(), think.as_ref(), act.is_some(), act.as_ref(), &reason,
        )
        .map_err(|e| { error!("ledger write failed — withholding response: {e}"); StatusCode::INTERNAL_SERVER_ERROR })?;
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
}

/// Forward request to upstream, stripping harness-specific headers.
/// Returns the raw reqwest::Response so the caller can inspect headers
/// before deciding whether to buffer or stream the body.
async fn send_upstream(
    client: &reqwest::Client,
    url: &str,
    headers: &HeaderMap,
    body: Bytes,
    strip: &[&str],
) -> Result<reqwest::Response> {
    let mut req = client.post(url).body(body);
    for (k, v) in headers.iter() {
        let name = k.as_str().to_lowercase();
        if name == "host" || name == "content-length" || strip.contains(&name.as_str()) {
            continue;
        }
        req = req.header(k.as_str(), v.as_bytes());
    }
    Ok(req.send().await?)
}

fn extract_openai(bytes: &[u8]) -> (String, Option<Value>, Option<Value>) {
    let Ok(v) = serde_json::from_slice::<Value>(bytes) else { return (String::new(), None, None) };
    let reason = v["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let act = v["choices"][0]["message"]["tool_calls"].clone();
    let act = if act.is_null() { None } else { Some(act) };
    // Grok-style reasoning_content. OpenAI o-series exposes only a token count in
    // usage.completion_tokens_details.reasoning_tokens — content is not returned by
    // the API. Ceiling: think will be null for standard GPT and OpenAI o-series.
    let think = v["choices"][0]["message"]["reasoning_content"].clone();
    let think = if think.is_null() { None } else { Some(think) };
    (reason, think, act)
}

fn extract_anthropic(bytes: &[u8]) -> (String, Option<Value>, Option<Value>) {
    let Ok(v) = serde_json::from_slice::<Value>(bytes) else { return (String::new(), None, None) };
    let mut reason = String::new();
    let mut think_blocks: Vec<Value> = Vec::new();
    let mut act = None;
    if let Some(content) = v["content"].as_array() {
        for block in content {
            match block["type"].as_str() {
                Some("text") => { reason = block["text"].as_str().unwrap_or("").to_string(); }
                Some("thinking") => { think_blocks.push(block.clone()); }
                Some("tool_use") => { act = Some(block.clone()); }
                _ => {}
            }
        }
    }
    let think = if think_blocks.is_empty() { None } else { Some(Value::Array(think_blocks)) };
    (reason, think, act)
}

/// Accumulate reasoning/text/tool content from an OpenAI SSE stream buffer.
/// Tool call input arrives as JSON delta fragments across multiple events;
/// we capture the presence marker but not the full reconstructed input —
/// documented ceiling for this iteration.
fn accumulate_sse_openai(buf: &[u8]) -> (String, Option<Value>, Option<Value>) {
    let text = std::str::from_utf8(buf).unwrap_or("");
    let mut reason = String::new();
    let mut thinking = String::new();
    let mut has_tool_calls = false;

    for line in text.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            if data.trim() == "[DONE]" {
                continue;
            }
            if let Ok(v) = serde_json::from_str::<Value>(data) {
                let delta = &v["choices"][0]["delta"];
                if let Some(c) = delta["content"].as_str() {
                    reason.push_str(c);
                }
                // Grok-style streaming reasoning_content
                if let Some(r) = delta["reasoning_content"].as_str() {
                    thinking.push_str(r);
                }
                if !delta["tool_calls"].is_null() {
                    has_tool_calls = true;
                }
            }
        }
    }

    let think = if thinking.is_empty() { None } else { Some(Value::String(thinking)) };
    // For streaming tool calls, record presence only — full reconstruction is future work
    let act = if has_tool_calls { Some(Value::String("[tool_calls — see raw stream]".into())) } else { None };
    (reason, think, act)
}

/// Accumulate reasoning/text/tool content from an Anthropic SSE stream buffer.
fn accumulate_sse_anthropic(buf: &[u8]) -> (String, Option<Value>, Option<Value>) {
    let text = std::str::from_utf8(buf).unwrap_or("");
    let mut reason = String::new();
    let mut thinking = String::new();
    let mut act: Option<Value> = None;

    for line in text.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            if let Ok(v) = serde_json::from_str::<Value>(data) {
                match v["type"].as_str() {
                    Some("content_block_delta") => {
                        match v["delta"]["type"].as_str() {
                            Some("text_delta") => {
                                if let Some(t) = v["delta"]["text"].as_str() {
                                    reason.push_str(t);
                                }
                            }
                            Some("thinking_delta") => {
                                if let Some(t) = v["delta"]["thinking"].as_str() {
                                    thinking.push_str(t);
                                }
                            }
                            _ => {}
                        }
                    }
                    Some("content_block_start") => {
                        if v["content_block"]["type"].as_str() == Some("tool_use") {
                            act = Some(v["content_block"].clone());
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    let think = if thinking.is_empty() { None } else { Some(Value::String(thinking)) };
    (reason, think, act)
}
