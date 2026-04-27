//! Configurable fake MCP upstream used by the proxy test suite.
//!
//! Speaks the Streamable HTTP transport (POST + GET + DELETE on `/`),
//! plus a small set of test-only control endpoints under `/__test/` for
//! injecting state changes from outside the MCP protocol.
//!
//! Hand-rolled (no `rmcp`) so we have full control over auth gating,
//! header inspection, dynamic tool/resource swaps, and forced
//! `notifications/*/list_changed` emission.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    body::Bytes,
    extract::{Json, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
    routing::{get, post},
};
use dashmap::DashMap;
use futures::stream;
use objectiveai::mcp::{
    JsonRpcError, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
    initialize_result::{
        Implementation, InitializeResult, ResourcesCapability, ServerCapabilities,
        ToolsCapability,
    },
    resource::{
        ListResourcesResult, ReadResourceRequestParams, ReadResourceResult, Resource,
    },
    shared::{ResourceContents, ResourceContentsUnion, TextResourceContents},
    tool::{
        CallToolRequestParams, CallToolResult, ContentBlock, ListToolsResult,
        TextContent, Tool, ToolSchema, ToolSchemaType,
    },
};
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, broadcast};
use tokio_util::sync::CancellationToken;

const PROTOCOL_VERSION: &str = "2025-11-25";
const SESSION_HEADER: &str = "Mcp-Session-Id";
const OUTBOUND_CAPACITY: usize = 64;

/// Knobs the test rig sets to shape the upstream's behavior.
#[derive(Debug, Clone)]
pub struct Config {
    /// Bind address. Pass `127.0.0.1:0` and read the assigned port from
    /// the returned [`Handle`].
    pub address: SocketAddr,
    /// Value placed in `initialize_result.server_info.name`.
    pub server_name: String,
    /// Initial set of tools to advertise. Replaceable at runtime via
    /// the `/__test/set-tools` endpoint.
    pub initial_tools: Vec<TestTool>,
    /// Initial set of resources. Replaceable via `/__test/set-resources`.
    pub initial_resources: Vec<TestResource>,
    /// If `Some`, every MCP request must carry `Authorization: <value>`
    /// exactly. Otherwise the request is rejected with HTTP 401.
    pub require_auth: Option<String>,
    /// `(header_name, expected_value)`. If set, every MCP request must
    /// carry that header with that exact value, else HTTP 400.
    pub header_gate: Option<(String, String)>,
}

/// One test tool. The `behavior` field decides what `tools/call` returns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestTool {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub behavior: TestToolBehavior,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TestToolBehavior {
    /// Echo the call's `arguments` back as a JSON-stringified text block.
    Echo,
    /// Sleep for `duration_ms` then return `arguments` as text. Used by
    /// the cancellation test.
    SleepThenEcho { duration_ms: u64 },
    /// Return a fixed string regardless of the input.
    Static { reply: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResource {
    pub uri: String,
    #[serde(default)]
    pub name: Option<String>,
    pub text: String,
}

/// Returned by [`spawn_test_upstream`]. Drop the `shutdown` token to
/// gracefully terminate the server.
#[derive(Debug)]
pub struct Handle {
    /// The actual address the server bound to (useful when caller passed `:0`).
    pub address: SocketAddr,
    /// Convenience: `http://<addr>/`.
    pub url: String,
    /// Convenience: `http://<addr>/__test`.
    pub control_base: String,
    /// Cancel to shut down the server; the `serve_task` finishes shortly after.
    pub shutdown: CancellationToken,
    /// Awaitable task running `axum::serve`. Tests can poll it on shutdown
    /// to confirm clean exit.
    pub serve_task: tokio::task::JoinHandle<anyhow::Result<()>>,
}

#[derive(Debug, Default)]
struct UpstreamState {
    tools: Vec<TestTool>,
    resources: Vec<TestResource>,
    /// Per-session SSE broadcast senders. The GET handler subscribes;
    /// `set_tools` / `set_resources` publish to every entry.
    sessions: HashMap<String, broadcast::Sender<JsonRpcNotification>>,
    /// Headers received on the most recent MCP POST. Tests assert against
    /// this via `GET /__test/seen-headers`.
    last_seen_headers: HashMap<String, String>,
}

#[derive(Clone)]
struct AppState {
    config: Arc<Config>,
    state: Arc<RwLock<UpstreamState>>,
    /// In-flight cancellation tokens keyed by serialized JSON-RPC request id.
    /// Used by the cancellation behavior so SleepThenEcho can be aborted.
    in_flight: Arc<DashMap<String, CancellationToken>>,
}

/// Spawn the test upstream on a tokio task. Returns immediately once the
/// listener is bound.
pub async fn spawn_test_upstream(config: Config) -> anyhow::Result<Handle> {
    let listener = tokio::net::TcpListener::bind(config.address).await?;
    let bound = listener.local_addr()?;
    let url = format!("http://{bound}/");
    let control_base = format!("http://{bound}/__test");

    let app_state = AppState {
        config: Arc::new(config.clone()),
        state: Arc::new(RwLock::new(UpstreamState {
            tools: config.initial_tools.clone(),
            resources: config.initial_resources.clone(),
            sessions: HashMap::new(),
            last_seen_headers: HashMap::new(),
        })),
        in_flight: Arc::new(DashMap::new()),
    };

    let router = axum::Router::new()
        .route(
            "/",
            post(handle_post).get(handle_get).delete(handle_delete),
        )
        .route("/__test/set-tools", post(set_tools_endpoint))
        .route("/__test/set-resources", post(set_resources_endpoint))
        .route("/__test/seen-headers", get(seen_headers_endpoint))
        .with_state(app_state);

    let shutdown = CancellationToken::new();
    let shutdown_clone = shutdown.clone();
    let serve_task = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async move { shutdown_clone.cancelled_owned().await })
            .await
            .map_err(anyhow::Error::from)
    });

    Ok(Handle {
        address: bound,
        url,
        control_base,
        shutdown,
        serve_task,
    })
}

// ---- MCP POST handler ----------------------------------------------------

async fn handle_post(
    State(app): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Err(resp) = check_gates(&app.config, &headers) {
        return resp;
    }
    record_seen_headers(&app, &headers).await;

    let value: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => return text_error(StatusCode::BAD_REQUEST, format!("invalid JSON: {e}")),
    };

    if value.get("id").is_none() {
        // Notification — no response, no further action.
        return StatusCode::ACCEPTED.into_response();
    }

    let request: JsonRpcRequest = match serde_json::from_value(value) {
        Ok(r) => r,
        Err(e) => {
            return text_error(StatusCode::BAD_REQUEST, format!("bad JSON-RPC: {e}"))
        }
    };

    match request.method.as_str() {
        "initialize" => initialize(&app, request).await,
        "ping" => ping(request),
        "tools/list" => tools_list(&app, request).await,
        "tools/call" => tools_call(&app, request).await,
        "resources/list" => resources_list(&app, request).await,
        "resources/read" => resources_read(&app, request).await,
        other => method_not_found(request.id, other),
    }
}

async fn initialize(app: &AppState, request: JsonRpcRequest) -> Response {
    // Mint a session id; the proxy will adopt it on initialize, then send
    // it back on every subsequent request.
    let session_id = uuid::Uuid::new_v4().to_string();
    let (tx, _) = broadcast::channel(OUTBOUND_CAPACITY);
    app.state
        .write()
        .await
        .sessions
        .insert(session_id.clone(), tx);

    let result = InitializeResult {
        protocol_version: PROTOCOL_VERSION.into(),
        capabilities: ServerCapabilities {
            experimental: None,
            logging: None,
            completions: None,
            prompts: None,
            tools: Some(ToolsCapability {
                list_changed: Some(true),
            }),
            resources: Some(ResourcesCapability {
                subscribe: None,
                list_changed: Some(true),
            }),
            tasks: None,
        },
        server_info: Implementation {
            name: app.config.server_name.clone(),
            title: None,
            version: env!("CARGO_PKG_VERSION").into(),
            website_url: None,
            description: None,
            icons: None,
        },
        instructions: None,
        _meta: None,
    };

    let body = JsonRpcResponse::Success {
        jsonrpc: "2.0".into(),
        id: request.id,
        result,
    };
    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        SESSION_HEADER,
        HeaderValue::from_str(&session_id).expect("uuid is valid header value"),
    );
    (StatusCode::OK, response_headers, Json(body)).into_response()
}

fn ping(request: JsonRpcRequest) -> Response {
    let body: JsonRpcResponse<serde_json::Value> = JsonRpcResponse::Success {
        jsonrpc: "2.0".into(),
        id: request.id,
        result: serde_json::json!({}),
    };
    (StatusCode::OK, Json(body)).into_response()
}

async fn tools_list(app: &AppState, request: JsonRpcRequest) -> Response {
    let tools: Vec<Tool> = app
        .state
        .read()
        .await
        .tools
        .iter()
        .map(test_tool_to_tool)
        .collect();
    let result = ListToolsResult {
        tools,
        next_cursor: None,
        _meta: None,
    };
    let body = JsonRpcResponse::Success {
        jsonrpc: "2.0".into(),
        id: request.id,
        result,
    };
    (StatusCode::OK, Json(body)).into_response()
}

async fn tools_call(app: &AppState, request: JsonRpcRequest) -> Response {
    let params: CallToolRequestParams = match request.params.clone() {
        Some(v) => match serde_json::from_value(v) {
            Ok(p) => p,
            Err(e) => return jsonrpc_error_response(request.id, -32602, format!("bad params: {e}")),
        },
        None => return jsonrpc_error_response(request.id, -32602, "missing params".into()),
    };

    let tool = match app
        .state
        .read()
        .await
        .tools
        .iter()
        .find(|t| t.name == params.name)
        .cloned()
    {
        Some(t) => t,
        None => return jsonrpc_error_response(request.id, -32601, format!("tool not found: {}", params.name)),
    };

    // Register an in-flight token so the SleepThenEcho path can observe
    // local cancellation if the proxy drops the request.
    let token = CancellationToken::new();
    let id_key = serde_json::to_string(&request.id).unwrap_or_default();
    app.in_flight.insert(id_key.clone(), token.clone());

    let result = run_tool_behavior(&tool, &params, &token).await;
    app.in_flight.remove(&id_key);

    match result {
        Ok(call_result) => {
            let body = JsonRpcResponse::Success {
                jsonrpc: "2.0".into(),
                id: request.id,
                result: call_result,
            };
            (StatusCode::OK, Json(body)).into_response()
        }
        Err(message) => jsonrpc_error_response(request.id, -32603, message),
    }
}

async fn run_tool_behavior(
    tool: &TestTool,
    params: &CallToolRequestParams,
    token: &CancellationToken,
) -> Result<CallToolResult, String> {
    let body_text = match &tool.behavior {
        TestToolBehavior::Echo => serde_json::to_string(&params.arguments)
            .unwrap_or_else(|_| "null".into()),
        TestToolBehavior::Static { reply } => reply.clone(),
        TestToolBehavior::SleepThenEcho { duration_ms } => {
            let dur = std::time::Duration::from_millis(*duration_ms);
            tokio::select! {
                _ = tokio::time::sleep(dur) => {}
                _ = token.cancelled() => return Err("cancelled".into()),
            }
            serde_json::to_string(&params.arguments).unwrap_or_else(|_| "null".into())
        }
    };

    Ok(CallToolResult {
        content: vec![ContentBlock::Text(TextContent {
            text: body_text,
            annotations: None,
            _meta: None,
        })],
        structured_content: None,
        is_error: None,
        _meta: None,
    })
}

async fn resources_list(app: &AppState, request: JsonRpcRequest) -> Response {
    let resources: Vec<Resource> = app
        .state
        .read()
        .await
        .resources
        .iter()
        .map(test_resource_to_resource)
        .collect();
    let result = ListResourcesResult {
        resources,
        next_cursor: None,
        _meta: None,
    };
    let body = JsonRpcResponse::Success {
        jsonrpc: "2.0".into(),
        id: request.id,
        result,
    };
    (StatusCode::OK, Json(body)).into_response()
}

async fn resources_read(app: &AppState, request: JsonRpcRequest) -> Response {
    let params: ReadResourceRequestParams = match request.params.clone() {
        Some(v) => match serde_json::from_value(v) {
            Ok(p) => p,
            Err(e) => return jsonrpc_error_response(request.id, -32602, format!("bad params: {e}")),
        },
        None => return jsonrpc_error_response(request.id, -32602, "missing params".into()),
    };

    let resource = match app
        .state
        .read()
        .await
        .resources
        .iter()
        .find(|r| r.uri == params.uri)
        .cloned()
    {
        Some(r) => r,
        None => return jsonrpc_error_response(request.id, -32602, format!("resource not found: {}", params.uri)),
    };

    let result = ReadResourceResult {
        contents: vec![ResourceContentsUnion::Text(TextResourceContents {
            base: ResourceContents {
                uri: resource.uri,
                mime_type: Some("text/plain".into()),
                _meta: None,
            },
            text: resource.text,
        })],
        _meta: None,
    };
    let body = JsonRpcResponse::Success {
        jsonrpc: "2.0".into(),
        id: request.id,
        result,
    };
    (StatusCode::OK, Json(body)).into_response()
}

// ---- MCP GET handler (SSE stream) ---------------------------------------

async fn handle_get(
    State(app): State<AppState>,
    headers: HeaderMap,
) -> Response {
    if let Err(resp) = check_gates(&app.config, &headers) {
        return resp;
    }
    let session_id = match session_id_from_headers(&headers) {
        Some(id) => id,
        None => return text_error(StatusCode::NOT_FOUND, "missing session id".into()),
    };
    let rx = match app.state.read().await.sessions.get(&session_id) {
        Some(tx) => tx.subscribe(),
        None => return text_error(StatusCode::NOT_FOUND, "unknown session".into()),
    };

    let stream = stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(notification) => {
                    let event = Event::default()
                        .json_data(&notification)
                        .ok()?;
                    return Some((Ok::<_, std::convert::Infallible>(event), rx));
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    });

    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(std::time::Duration::from_secs(15)))
        .into_response()
}

// ---- MCP DELETE handler -------------------------------------------------

async fn handle_delete(
    State(app): State<AppState>,
    headers: HeaderMap,
) -> Response {
    let session_id = match session_id_from_headers(&headers) {
        Some(id) => id,
        None => return text_error(StatusCode::NOT_FOUND, "missing session id".into()),
    };
    let removed = app.state.write().await.sessions.remove(&session_id).is_some();
    if removed {
        StatusCode::OK.into_response()
    } else {
        text_error(StatusCode::NOT_FOUND, "unknown session".into())
    }
}

// ---- Test-only control endpoints ----------------------------------------

#[derive(Debug, Deserialize)]
struct SetToolsBody {
    tools: Vec<TestTool>,
}

async fn set_tools_endpoint(
    State(app): State<AppState>,
    Json(body): Json<SetToolsBody>,
) -> Response {
    {
        let mut state = app.state.write().await;
        state.tools = body.tools;
        let notification = JsonRpcNotification {
            jsonrpc: "2.0".into(),
            method: "notifications/tools/list_changed".into(),
            params: None,
        };
        for tx in state.sessions.values() {
            let _ = tx.send(notification.clone());
        }
    }
    StatusCode::OK.into_response()
}

#[derive(Debug, Deserialize)]
struct SetResourcesBody {
    resources: Vec<TestResource>,
}

async fn set_resources_endpoint(
    State(app): State<AppState>,
    Json(body): Json<SetResourcesBody>,
) -> Response {
    {
        let mut state = app.state.write().await;
        state.resources = body.resources;
        let notification = JsonRpcNotification {
            jsonrpc: "2.0".into(),
            method: "notifications/resources/list_changed".into(),
            params: None,
        };
        for tx in state.sessions.values() {
            let _ = tx.send(notification.clone());
        }
    }
    StatusCode::OK.into_response()
}

async fn seen_headers_endpoint(State(app): State<AppState>) -> Response {
    let headers = app.state.read().await.last_seen_headers.clone();
    Json(headers).into_response()
}

// ---- Helpers ------------------------------------------------------------

fn check_gates(config: &Config, headers: &HeaderMap) -> Result<(), Response> {
    if let Some(expected_auth) = &config.require_auth {
        let actual = headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if actual != expected_auth {
            return Err(text_error(StatusCode::UNAUTHORIZED, "auth required".into()));
        }
    }
    if let Some((name, expected)) = &config.header_gate {
        let actual = headers.get(name.as_str()).and_then(|v| v.to_str().ok()).unwrap_or("");
        if actual != expected.as_str() {
            return Err(text_error(
                StatusCode::BAD_REQUEST,
                format!("required header {name} missing or wrong value"),
            ));
        }
    }
    Ok(())
}

async fn record_seen_headers(app: &AppState, headers: &HeaderMap) {
    let mut snapshot = HashMap::new();
    for (name, value) in headers.iter() {
        if let Ok(v) = value.to_str() {
            snapshot.insert(name.as_str().to_string(), v.to_string());
        }
    }
    app.state.write().await.last_seen_headers = snapshot;
}

fn session_id_from_headers(headers: &HeaderMap) -> Option<String> {
    headers.get(SESSION_HEADER)?.to_str().ok().map(String::from)
}

fn text_error(status: StatusCode, message: String) -> Response {
    (status, message).into_response()
}

fn jsonrpc_error_response(
    id: serde_json::Value,
    code: i64,
    message: String,
) -> Response {
    let body: JsonRpcResponse<()> = JsonRpcResponse::Error {
        jsonrpc: "2.0".into(),
        id,
        error: JsonRpcError {
            code,
            message,
            data: None,
        },
    };
    (StatusCode::OK, Json(body)).into_response()
}

fn method_not_found(id: serde_json::Value, method: &str) -> Response {
    jsonrpc_error_response(id, -32601, format!("method not found: {method}"))
}

fn test_tool_to_tool(t: &TestTool) -> Tool {
    Tool {
        name: t.name.clone(),
        title: None,
        description: t.description.clone(),
        icons: None,
        input_schema: ToolSchema {
            r#type: ToolSchemaType::Object,
            properties: None,
            required: None,
            extra: indexmap::IndexMap::new(),
        },
        output_schema: None,
        annotations: None,
        execution: None,
        _meta: None,
    }
}

fn test_resource_to_resource(r: &TestResource) -> Resource {
    Resource {
        name: r.name.clone().unwrap_or_else(|| r.uri.clone()),
        title: None,
        uri: r.uri.clone(),
        description: None,
        mime_type: Some("text/plain".into()),
        icons: None,
        annotations: None,
        _meta: None,
    }
}
