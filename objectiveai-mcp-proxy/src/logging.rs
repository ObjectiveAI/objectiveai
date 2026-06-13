//! Optional request/response JSONL logging for the proxy.
//!
//! Enabled when `OBJECTIVEAI_LOGS_DIR` is set (or the embedding API
//! passes [`crate::ConfigBuilder::logs_dir`]). Every HTTP request
//! through the proxy appends one line to `<logs_dir>/mcp-proxy.jsonl`
//! capturing, in order:
//!
//! - `timestamp` — milliseconds since the Unix epoch,
//! - `session_id` — the `Mcp-Session-Id` (request header, else the
//!   response header `initialize` minted),
//! - `request` — method, path, headers, body,
//! - `response` — status, headers, body.
//!
//! Bodies are parsed as JSON when possible (so a `tools/list` result is
//! readable inline), otherwise stored as a UTF-8 string. `text/event-
//! stream` responses (the long-lived GET notification stream and any
//! SSE POST reply) are logged without their body — buffering a live
//! stream would hang it.
//!
//! Writes are append-only via tokio fs; the log directory is created on
//! demand each write. Every failure is swallowed — diagnostics must
//! never break the proxy.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    body::{Body, to_bytes},
    extract::{Request, State},
    http::{HeaderMap, header},
    middleware::Next,
    response::Response,
};
use serde::Serialize;
use tokio::io::AsyncWriteExt;

/// `Mcp-Session-Id` — `HeaderMap` lookups are case-insensitive, so the
/// lowercase spelling matches whatever casing the client sent.
const SESSION_ID_HEADER: &str = "mcp-session-id";

/// Append-only JSONL writer for proxy traffic.
pub struct ProxyLogger {
    dir: PathBuf,
    path: PathBuf,
}

impl ProxyLogger {
    pub fn new(dir: PathBuf) -> Self {
        let path = dir.join("mcp-proxy.jsonl");
        Self { dir, path }
    }

    /// Append one serialized [`LogLine`] + newline. Creates the log
    /// directory if it doesn't exist. All errors are swallowed.
    async fn append(&self, line: &LogLine) {
        let Ok(mut json) = serde_json::to_string(line) else {
            return;
        };
        json.push('\n');
        if tokio::fs::create_dir_all(&self.dir).await.is_err() {
            return;
        }
        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await;
        if let Ok(mut file) = file {
            let _ = file.write_all(json.as_bytes()).await;
        }
    }
}

#[derive(Serialize)]
struct LogLine {
    /// Milliseconds since the Unix epoch when the line was written.
    timestamp: u64,
    session_id: Option<String>,
    request: ReqLog,
    response: RespLog,
}

#[derive(Serialize)]
struct ReqLog {
    method: String,
    path: String,
    headers: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct RespLog {
    status: u16,
    headers: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<serde_json::Value>,
}

/// `HeaderMap` → sorted name→value map, lossy for non-UTF8 values.
fn headers_map(headers: &HeaderMap) -> BTreeMap<String, String> {
    headers
        .iter()
        .map(|(k, v)| {
            (
                k.as_str().to_string(),
                String::from_utf8_lossy(v.as_bytes()).into_owned(),
            )
        })
        .collect()
}

/// Parse a body as JSON for readability; fall back to a UTF-8 string.
/// Empty bodies become `None`.
fn body_value(bytes: &[u8]) -> Option<serde_json::Value> {
    if bytes.is_empty() {
        return None;
    }
    match serde_json::from_slice(bytes) {
        Ok(v) => Some(v),
        Err(_) => Some(serde_json::Value::String(
            String::from_utf8_lossy(bytes).into_owned(),
        )),
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn session_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get(SESSION_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// axum middleware (via `from_fn_with_state`) that appends one JSONL
/// line per request. Buffers the request body and — for non-SSE
/// responses — the response body, reconstructing both so the handler
/// chain is unaffected.
pub async fn log_layer(
    State(logger): State<Arc<ProxyLogger>>,
    req: Request,
    next: Next,
) -> Response {
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let req_headers = headers_map(req.headers());
    let req_session_id = session_id(req.headers());

    // Buffer + restore the request body.
    let (parts, body) = req.into_parts();
    let req_bytes = to_bytes(body, usize::MAX).await.unwrap_or_default();
    let req = Request::from_parts(parts, Body::from(req_bytes.clone()));

    let response = next.run(req).await;

    let status = response.status().as_u16();
    let resp_headers = headers_map(response.headers());
    // `initialize` mints the session id on the response, not the request.
    let session_id = req_session_id.or_else(|| session_id(response.headers()));
    let is_event_stream = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|ct| ct.contains("text/event-stream"));

    let request = ReqLog {
        method,
        path,
        headers: req_headers,
        body: body_value(&req_bytes),
    };

    if is_event_stream {
        // Live stream — buffering the body would hang it.
        let line = LogLine {
            timestamp: now_millis(),
            session_id,
            request,
            response: RespLog {
                status,
                headers: resp_headers,
                body: None,
            },
        };
        logger.append(&line).await;
        response
    } else {
        let (parts, body) = response.into_parts();
        let resp_bytes = to_bytes(body, usize::MAX).await.unwrap_or_default();
        let line = LogLine {
            timestamp: now_millis(),
            session_id,
            request,
            response: RespLog {
                status,
                headers: resp_headers,
                body: body_value(&resp_bytes),
            },
        };
        logger.append(&line).await;
        Response::from_parts(parts, Body::from(resp_bytes))
    }
}
