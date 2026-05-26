//! `ConduitMcpHandler` — reverse-attach MCP forwarder.
//!
//! Inlined into `objectiveai-cli-stream` so this crate has no dep
//! on `objectiveai-cli`. The implementation mirrors
//! `objectiveai-cli/src/api/conduit.rs` 1:1 (the canonical version
//! will be consolidated into a shared home — likely the SDK — when
//! `objectiveai-cli`'s streaming subcommands are removed and its
//! copy goes with them).
//!
//! Dispatch on an inbound `server_request`:
//! - **No `Mcp-Session-Id` header (fresh `initialize`).** Dial the
//!   remote with `session_id = None`; the remote mints one and we
//!   key the new [`Connection`] under it. The synthesized
//!   `initialize` response stamps that id back in the response
//!   `Mcp-Session-Id` header so the proxy adopts it.
//! - **Header present + already in the map.** Reuse the cached
//!   [`Connection`].
//! - **Header present + not in the map (continuation resume).**
//!   Dial the remote with `session_id = Some(incoming)`. The SDK
//!   handles the resume branch — many servers don't echo the
//!   header back on resume, so the SDK falls back to the caller's
//!   provided id.
//!
//! Then:
//! - `initialize` → synthesize from `connection.initialize_result`
//!   (the SDK already handshook on `connect`). Strip
//!   `{tools,resources}.listChanged` from advertised capabilities
//!   so the proxy never subscribes — the chain stays single-shot.
//! - notifications (no `id`) → 202 Accepted, no body, never round-trip.
//! - everything else → raw POST through `connection.http_client` +
//!   `connection.url` + `connection.session_id`. Response parsed by
//!   [`parse_json_or_sse`] (rmcp's `StreamableHttpService` may pick
//!   either shape).

use dashmap::DashMap;
use indexmap::IndexMap;
use objectiveai_sdk::client_objectiveai_mcp::{server_request, server_response};
use objectiveai_sdk::http::McpHandler;
use std::sync::Arc;
use std::time::Duration;

struct ConduitState {
    connection: objectiveai_sdk::mcp::Connection,
}

#[derive(Clone)]
pub struct ConduitMcpHandler {
    inner: Arc<Inner>,
}

struct Inner {
    /// Configured remote MCP URL (e.g. `https://mcp.example.com`).
    /// `None` ⇒ MCP isn't configured for this invocation; every
    /// request 501s the same way `objectiveai_sdk::http::RejectHandler`
    /// would.
    mcp_url: Option<String>,
    client: objectiveai_sdk::mcp::Client,
    connections: DashMap<String, Arc<ConduitState>>,
}

impl ConduitMcpHandler {
    /// Construct a handler that dials the given URL on first use.
    /// `None` makes every `handle()` call reject with 501.
    pub fn new(mcp_url: Option<String>) -> Self {
        let http = reqwest::Client::builder()
            .build()
            .expect("reqwest::Client::build is infallible without rustls toggles");
        let client = objectiveai_sdk::mcp::Client::new(
            http,
            "objectiveai-cli-stream-conduit".to_string(),
            String::new(),
            String::new(),
            Duration::from_secs(30),
            Duration::from_secs(1),
            Duration::from_secs(1),
            0.5,
            2.0,
            Duration::from_secs(30),
            Duration::from_secs(30),
            Duration::from_secs(60),
        );
        Self {
            inner: Arc::new(Inner {
                mcp_url,
                client,
                connections: DashMap::new(),
            }),
        }
    }

    async fn dial(
        &self,
        url: String,
        session_id: Option<String>,
        request_headers: &IndexMap<String, String>,
    ) -> Result<Arc<ConduitState>, objectiveai_sdk::mcp::Error> {
        let connect_headers = sanitize_connect_headers(request_headers);
        let connection = self
            .inner
            .client
            .connect(url, session_id, Some(connect_headers))
            .await?;
        Ok(Arc::new(ConduitState { connection }))
    }
}

impl McpHandler for ConduitMcpHandler {
    async fn handle(&self, request: server_request::Request) -> server_response::Response {
        let id_for_err = request.id.clone();

        let Some(mcp_url) = self.inner.mcp_url.as_ref() else {
            return reject_no_mcp(id_for_err);
        };

        let incoming_session_id: Option<String> = request
            .headers
            .iter()
            .find_map(|(k, v)| {
                k.eq_ignore_ascii_case("mcp-session-id").then(|| v.clone())
            });

        let state = match &incoming_session_id {
            Some(sid) => {
                if let Some(existing) = self.inner.connections.get(sid) {
                    existing.clone()
                } else {
                    let dial_result = self
                        .dial(mcp_url.clone(), Some(sid.clone()), &request.headers)
                        .await;
                    match dial_result {
                        Ok(st) => {
                            self.inner.connections.insert(sid.clone(), st.clone());
                            st
                        }
                        Err(e) => {
                            return conduit_error(id_for_err, format!("connect (resume): {e}"));
                        }
                    }
                }
            }
            None => {
                let dial_result = self.dial(mcp_url.clone(), None, &request.headers).await;
                match dial_result {
                    Ok(st) => {
                        self.inner
                            .connections
                            .insert(st.connection.session_id.clone(), st.clone());
                        st
                    }
                    Err(e) => {
                        return conduit_error(id_for_err, format!("connect: {e}"));
                    }
                }
            }
        };

        match forward(&state, request).await {
            Ok(resp) => resp,
            Err(e) => conduit_error(id_for_err, e.to_string()),
        }
    }
}

/// Hop-by-hop and layer-internal headers don't propagate to MCP.
fn sanitize_connect_headers(
    headers: &IndexMap<String, String>,
) -> IndexMap<String, String> {
    let mut out = headers.clone();
    for k in [
        "Host",
        "host",
        "Content-Length",
        "content-length",
        "Mcp-Session-Id",
        "mcp-session-id",
    ] {
        out.shift_remove(k);
    }
    out
}

async fn forward(
    state: &ConduitState,
    request: server_request::Request,
) -> Result<server_response::Response, ConduitError> {
    let envelope = request.body.clone();

    let rpc_id = envelope.as_ref().and_then(|v| v.get("id")).cloned();
    let rpc_method = envelope
        .as_ref()
        .and_then(|v| v.get("method"))
        .and_then(|m| m.as_str())
        .map(|s| s.to_string());

    // Notifications (no `id`) → 202 with no body; don't round-trip.
    if rpc_id.is_none() {
        return Ok(server_response::Response {
            id: request.id,
            status: 202,
            headers: IndexMap::new(),
            body: None,
        });
    }

    // `initialize`: synthesize from the SDK Connection's cached
    // InitializeResult; don't re-handshake. Stamp the remote-minted
    // session id on the response so the proxy adopts it.
    if rpc_method.as_deref() == Some("initialize") {
        let mut init_value = serde_json::to_value(&state.connection.initialize_result)
            .map_err(ConduitError::Serialize)?;
        if let Some(caps) = init_value.pointer_mut("/capabilities") {
            if let Some(obj) = caps.as_object_mut() {
                if let Some(tools) = obj.get_mut("tools").and_then(|t| t.as_object_mut()) {
                    tools.remove("listChanged");
                }
                if let Some(resources) =
                    obj.get_mut("resources").and_then(|r| r.as_object_mut())
                {
                    resources.remove("listChanged");
                }
            }
        }
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": rpc_id.unwrap(),
            "result": init_value,
        });
        let mut headers = IndexMap::new();
        headers.insert(
            "Mcp-Session-Id".to_string(),
            state.connection.session_id.clone(),
        );
        return Ok(server_response::Response {
            id: request.id,
            status: 200,
            headers,
            body: Some(body),
        });
    }

    // Everything else: raw POST through the Connection.
    let conn = &state.connection;
    let mut req = conn.http_client.post(&conn.url);
    for (k, v) in &request.headers {
        if k.eq_ignore_ascii_case("host")
            || k.eq_ignore_ascii_case("content-length")
            || k.eq_ignore_ascii_case("connection")
            || k.eq_ignore_ascii_case("accept")
            || k.eq_ignore_ascii_case("content-type")
            || k.eq_ignore_ascii_case("mcp-session-id")
        {
            continue;
        }
        req = req.header(k, v);
    }
    req = req
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("Mcp-Session-Id", &conn.session_id);
    if let Some(body) = envelope.as_ref() {
        req = req.json(body);
    }

    let resp = req.send().await.map_err(ConduitError::Request)?;
    let status = resp.status().as_u16();
    let mut resp_headers = IndexMap::new();
    for (k, v) in resp.headers().iter() {
        if k.as_str().eq_ignore_ascii_case("mcp-session-id")
            || k.as_str().eq_ignore_ascii_case("content-type")
            || k.as_str().eq_ignore_ascii_case("transfer-encoding")
            || k.as_str().eq_ignore_ascii_case("content-length")
        {
            // mcp-session-id: local-layer; the API uses the conduit's
            // real session id stamped elsewhere.
            // content-type: the API re-sets it on body presence.
            // transfer-encoding / content-length: framing headers
            // scoped to the conduit↔objectiveai-mcp TCP connection.
            continue;
        }
        if let Ok(value) = v.to_str() {
            resp_headers.insert(k.as_str().to_string(), value.to_string());
        }
    }
    let resp_text = resp.text().await.map_err(ConduitError::Body)?;
    let resp_body = parse_json_or_sse(&resp_text);

    Ok(server_response::Response {
        id: request.id,
        status,
        headers: resp_headers,
        body: resp_body,
    })
}

fn reject_no_mcp(id: String) -> server_response::Response {
    server_response::Response {
        id,
        status: 501,
        headers: IndexMap::new(),
        body: Some(serde_json::json!({
            "jsonrpc": "2.0",
            "id": serde_json::Value::Null,
            "error": {
                "code": -32601,
                "message": "this client has no MCP server configured (pass --mcp-address)",
            },
        })),
    }
}

fn conduit_error(id: String, message: impl Into<String>) -> server_response::Response {
    let message = message.into();
    server_response::Response {
        id,
        status: 502,
        headers: IndexMap::new(),
        body: Some(serde_json::json!({
            "jsonrpc": "2.0",
            "id": serde_json::Value::Null,
            "error": {
                "code": -32603,
                "message": format!("conduit: {message}"),
            },
        })),
    }
}

/// Parses bare JSON; falls back to stripping `data:` prefixes and
/// reparsing for SSE-wrapped responses.
fn parse_json_or_sse(text: &str) -> Option<serde_json::Value> {
    if text.is_empty() {
        return None;
    }
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(text) {
        return Some(v);
    }
    let collected: String = text
        .lines()
        .filter_map(|l| l.strip_prefix("data: ").or_else(|| l.strip_prefix("data:")))
        .collect();
    if collected.is_empty() {
        return None;
    }
    serde_json::from_str::<serde_json::Value>(&collected).ok()
}

#[derive(Debug, thiserror::Error)]
enum ConduitError {
    #[error("forwarding HTTP request failed: {0}")]
    Request(reqwest::Error),
    #[error("reading response body failed: {0}")]
    Body(reqwest::Error),
    #[error("serializing InitializeResult failed: {0}")]
    Serialize(serde_json::Error),
}
