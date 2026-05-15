mod jcs;
mod ledger;
mod ulid;

use anyhow::Result;
use axum::{
    Router,
    body::Body,
    extract::{OriginalUri, Path as PathParam, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::Response,
    routing::post,
};
use bytes::Bytes;
use futures_util::StreamExt as _;
use ledger::SessionLedger;
use serde_json::Value;
use std::{collections::HashMap, path::PathBuf, str::FromStr, sync::Arc};
use tokio_stream::wrappers::ReceiverStream;
use tracing::{error, info};

const SESSION_HEADER: &str = "x-harness-session";
const UPSTREAM_HEADER: &str = "x-harness-upstream";

#[derive(Clone)]
struct AppState {
    harness_root: PathBuf,
    upstream_base: String,
    anthropic_base: String,
    gemini_base: String,
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
    let gemini_base = std::env::var("GEMINI_BASE_URL")
        .unwrap_or_else(|_| "https://generativelanguage.googleapis.com".to_string())
        .trim_end_matches('/')
        .to_string();

    let listen = std::env::var("HARNESS_LISTEN")
        .unwrap_or_else(|_| "127.0.0.1:8474".to_string());

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .connection_verbose(false)
        .build()?;

    let state = Arc::new(AppState {
        harness_root,
        upstream_base,
        anthropic_base,
        gemini_base,
        client,
    });

    let app = Router::new()
        .route("/v1/chat/completions", post(openai_handler))
        .route("/v1/messages", post(anthropic_handler))
        .route("/v1beta/models/*model", post(gemini_handler))
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
    let mut tool_use_blocks: Vec<Value> = Vec::new();
    if let Some(content) = v["content"].as_array() {
        for block in content {
            match block["type"].as_str() {
                Some("text") => { reason = block["text"].as_str().unwrap_or("").to_string(); }
                Some("thinking") => { think_blocks.push(block.clone()); }
                Some("tool_use") => { tool_use_blocks.push(block.clone()); }
                _ => {}
            }
        }
    }
    let think = if think_blocks.is_empty() { None } else { Some(Value::Array(think_blocks)) };
    let act = match tool_use_blocks.len() {
        0 => None,
        1 => tool_use_blocks.into_iter().next(),
        _ => Some(Value::Array(tool_use_blocks)),
    };
    (reason, think, act)
}

/// Accumulate reasoning/text/tool content from an OpenAI SSE stream buffer.
/// Tool call inputs arrive as `arguments` delta fragments across multiple events,
/// keyed by `tool_calls[*].index`. Reconstructs full tool call array.
fn accumulate_sse_openai(buf: &[u8]) -> (String, Option<Value>, Option<Value>) {
    let text = std::str::from_utf8(buf).unwrap_or("");
    let mut reason = String::new();
    let mut thinking = String::new();
    // index -> (id, name, accumulated_arguments_string)
    let mut tool_calls: HashMap<usize, (String, String, String)> = HashMap::new();

    for line in text.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            if data.trim() == "[DONE]" { continue; }
            if let Ok(v) = serde_json::from_str::<Value>(data) {
                let delta = &v["choices"][0]["delta"];
                if let Some(c) = delta["content"].as_str() {
                    reason.push_str(c);
                }
                // Grok-style streaming reasoning_content
                if let Some(r) = delta["reasoning_content"].as_str() {
                    thinking.push_str(r);
                }
                if let Some(tcs) = delta["tool_calls"].as_array() {
                    for tc in tcs {
                        let idx = tc["index"].as_u64().unwrap_or(0) as usize;
                        let entry = tool_calls.entry(idx)
                            .or_insert_with(|| (String::new(), String::new(), String::new()));
                        if let Some(id) = tc["id"].as_str() {
                            if entry.0.is_empty() { entry.0 = id.to_string(); }
                        }
                        if let Some(name) = tc["function"]["name"].as_str() {
                            if entry.1.is_empty() { entry.1 = name.to_string(); }
                        }
                        if let Some(args) = tc["function"]["arguments"].as_str() {
                            entry.2.push_str(args);
                        }
                    }
                }
            }
        }
    }

    let think = if thinking.is_empty() { None } else { Some(Value::String(thinking)) };
    let act = if tool_calls.is_empty() {
        None
    } else {
        let mut sorted: Vec<_> = tool_calls.into_iter().collect();
        sorted.sort_by_key(|(idx, _)| *idx);
        let arr: Vec<Value> = sorted.into_iter().map(|(_, (id, name, args))| {
            let arguments = serde_json::from_str::<Value>(&args)
                .unwrap_or(Value::String(args));
            serde_json::json!({
                "id": id,
                "type": "function",
                "function": { "name": name, "arguments": arguments }
            })
        }).collect();
        Some(Value::Array(arr))
    };
    (reason, think, act)
}

/// Accumulate reasoning/text/tool content from an Anthropic SSE stream buffer.
/// `input_json_delta` events carry partial JSON fragments for tool inputs;
/// we accumulate them per content-block index and parse the full JSON at stream end.
fn accumulate_sse_anthropic(buf: &[u8]) -> (String, Option<Value>, Option<Value>) {
    let text = std::str::from_utf8(buf).unwrap_or("");
    let mut reason = String::new();
    let mut thinking = String::new();
    // content block index -> (block Value from content_block_start, accumulated input JSON string)
    let mut tool_blocks: HashMap<usize, (Value, String)> = HashMap::new();

    for line in text.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            if let Ok(v) = serde_json::from_str::<Value>(data) {
                match v["type"].as_str() {
                    Some("content_block_delta") => {
                        let idx = v["index"].as_u64().unwrap_or(0) as usize;
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
                            Some("input_json_delta") => {
                                if let Some(entry) = tool_blocks.get_mut(&idx) {
                                    if let Some(partial) = v["delta"]["partial_json"].as_str() {
                                        entry.1.push_str(partial);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    Some("content_block_start") => {
                        if v["content_block"]["type"].as_str() == Some("tool_use") {
                            let idx = v["index"].as_u64().unwrap_or(0) as usize;
                            tool_blocks.insert(idx, (v["content_block"].clone(), String::new()));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    let think = if thinking.is_empty() { None } else { Some(Value::String(thinking)) };
    let act = if tool_blocks.is_empty() {
        None
    } else {
        let mut sorted: Vec<_> = tool_blocks.into_iter().collect();
        sorted.sort_by_key(|(idx, _)| *idx);
        let blocks: Vec<Value> = sorted.into_iter().map(|(_, (mut block, input_str))| {
            let input = if input_str.is_empty() {
                Value::Object(serde_json::Map::new())
            } else {
                serde_json::from_str::<Value>(&input_str)
                    .unwrap_or(Value::String(input_str))
            };
            if let Value::Object(ref mut map) = block {
                map.insert("input".to_string(), input);
            }
            block
        }).collect();
        if blocks.len() == 1 {
            blocks.into_iter().next()
        } else {
            Some(Value::Array(blocks))
        }
    };
    (reason, think, act)
}

/// Gemini-compatible handler. Route: /v1beta/models/*model
/// Covers both :generateContent (buffered) and :streamGenerateContent (SSE).
/// Query string (e.g. ?alt=sse) is forwarded verbatim via OriginalUri.
async fn gemini_handler(
    State(state): State<Arc<AppState>>,
    PathParam(model_path): PathParam<String>,
    OriginalUri(original_uri): OriginalUri,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, StatusCode> {
    let payload: Value = serde_json::from_slice(&body).map_err(|_| StatusCode::BAD_REQUEST)?;

    let sid = headers
        .get(SESSION_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
        .unwrap_or_else(ulid::new_ulid);

    // Gemini uses systemInstruction for system prompt; contents for messages.
    let system_str;
    let system = match payload.get("systemInstruction") {
        Some(v) => { system_str = v.to_string(); Some(system_str.as_str()) }
        None => None,
    };
    let in_hash = ledger::hash_input(
        system,
        payload.get("contents"),
        payload.get("tools"),
    );

    // Forward full path + query string so ?alt=sse reaches the Gemini endpoint.
    let upstream_url = format!(
        "{}{}",
        state.gemini_base,
        original_uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/")
    );
    // Strip method suffix for clean model name in the ledger.
    let model = model_path
        .trim_end_matches(":streamGenerateContent")
        .trim_end_matches(":generateContent")
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
            let (reason, think, act) = accumulate_sse_gemini(&buf);
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
        let (reason, think, act) = extract_gemini(&res_bytes);
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

fn extract_gemini(bytes: &[u8]) -> (String, Option<Value>, Option<Value>) {
    let Ok(v) = serde_json::from_slice::<Value>(bytes) else { return (String::new(), None, None) };
    let mut reason = String::new();
    let mut think_blocks: Vec<Value> = Vec::new();
    let mut fn_call_blocks: Vec<Value> = Vec::new();
    if let Some(parts) = v["candidates"][0]["content"]["parts"].as_array() {
        for part in parts {
            if part["thought"].as_bool() == Some(true) {
                think_blocks.push(part.clone());
            } else if let Some(fc) = part.get("functionCall").filter(|v| !v.is_null()) {
                fn_call_blocks.push(fc.clone());
            } else if let Some(t) = part["text"].as_str() {
                reason.push_str(t);
            }
        }
    }
    let think = if think_blocks.is_empty() { None } else { Some(Value::Array(think_blocks)) };
    let act = match fn_call_blocks.len() {
        0 => None,
        1 => fn_call_blocks.into_iter().next(),
        _ => Some(Value::Array(fn_call_blocks)),
    };
    (reason, think, act)
}

/// Accumulate from Gemini SSE stream. Each data: line is a complete
/// GenerateContentResponse chunk — accumulate parts across all chunks.
fn accumulate_sse_gemini(buf: &[u8]) -> (String, Option<Value>, Option<Value>) {
    let text = std::str::from_utf8(buf).unwrap_or("");
    let mut reason = String::new();
    let mut thinking = String::new();
    let mut fn_call_blocks: Vec<Value> = Vec::new();

    for line in text.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            if let Ok(v) = serde_json::from_str::<Value>(data) {
                if let Some(parts) = v["candidates"][0]["content"]["parts"].as_array() {
                    for part in parts {
                        if part["thought"].as_bool() == Some(true) {
                            if let Some(t) = part["text"].as_str() {
                                thinking.push_str(t);
                            }
                        } else if let Some(fc) = part.get("functionCall").filter(|v| !v.is_null()) {
                            fn_call_blocks.push(fc.clone());
                        } else if let Some(t) = part["text"].as_str() {
                            reason.push_str(t);
                        }
                    }
                }
            }
        }
    }

    let think = if thinking.is_empty() { None } else { Some(Value::String(thinking)) };
    let act = match fn_call_blocks.len() {
        0 => None,
        1 => fn_call_blocks.into_iter().next(),
        _ => Some(Value::Array(fn_call_blocks)),
    };
    (reason, think, act)
}
