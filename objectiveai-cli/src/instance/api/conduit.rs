//! `ConduitMcpHandler` — true medium for the proxy's per-MCP
//! Streamable HTTP requests. Each request the API forwards over the
//! WS reverse-attach channel carries a typed [`McpKind`]
//! discriminator that names exactly one upstream MCP server (the
//! local primary `objectiveai-mcp` for [`McpKind::ObjectiveAi`], or
//! a plugin-spawned MCP for [`McpKind::Other`]). The conduit dials
//! that upstream and forwards verbatim — no tool renaming, no
//! routing, no aggregation, no capability synthesis. The CLI is a
//! pass-through; capabilities, server name, and protocol version
//! all come from the upstream itself.
//!
//! Storage is a single `connections` map keyed by upstream's native
//! `Mcp-Session-Id`. The map survives across response_id boundaries
//! (multiple agents sharing the WS reuse the same upstream
//! connections via shared session ids), and reconstructs on cache
//! miss for the primary upstream by re-dialing with the old session
//! id. Plugin cache miss returns `-32001` and lets the proxy retry
//! with a fresh `initialize` — the plugin's disk state covers
//! server-side resume.
//!
//! `Notifier` is late-bound: the pump needs one, but the `Notifier`
//! is output of `send_streaming_ws(handler, ...)` and the handler is
//! input. The caller constructs the conduit, threads its clone into
//! `send_streaming_ws`, then calls [`ConduitMcpHandler::install_notifier`]
//! on the original handle once the notifier is in hand. Pump
//! closures read the slot at fire time; events that fire before
//! install are dropped (the window is bounded by a few statements
//! at stream startup).

use dashmap::DashMap;
use indexmap::IndexMap;
use objectiveai_sdk::Notifier;
use objectiveai_sdk::cli::plugins::{Output as PluginOutput, TypedOutput as TypedPluginOutput};
use objectiveai_sdk::client_objectiveai_mcp::McpKind;
use objectiveai_sdk::client_objectiveai_mcp::client_request::{McpListChanged, McpListChangedKind};
use objectiveai_sdk::client_objectiveai_mcp::server_response::{InitializeReply, JsonRpcResult};
use objectiveai_sdk::client_objectiveai_mcp::{server_request, server_response};
use objectiveai_sdk::http::McpHandler;
use objectiveai_sdk::mcp::resource::{
    ListResourcesRequest, ListResourcesResult, ReadResourceRequestParams, ReadResourceResult,
};
use objectiveai_sdk::mcp::tool::{
    CallToolRequestParams, CallToolResult, ListToolsRequest, ListToolsResult,
};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

struct ConduitState {
    connection: objectiveai_sdk::mcp::Connection,
    /// Which upstream this state addresses. Captured at dial time so
    /// the list-changed pump can stamp it on every
    /// [`McpListChanged`] frame.
    mcp_kind: McpKind,
    /// `X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY` of the request that dialed this
    /// upstream. Carried for wire-shape parity and diagnostic
    /// readability; the actual sweep key is `response_id` below.
    agent_instance_hierarchy: String,
    /// `X-OBJECTIVEAI-RESPONSE-ID` of the request that dialed this
    /// upstream — the per-agent-slot leaf the API minted at
    /// `proxy_request_headers` build time. Used by
    /// [`ConduitMcpHandler::select_response_ids`] for direct
    /// equality match against the streamed chunk's
    /// `agent_completion_ids()`.
    response_id: String,
}

#[derive(Clone)]
pub struct ConduitMcpHandler {
    inner: Arc<Inner>,
}

struct Inner {
    /// Configured remote MCP URL for the primary `objectiveai-mcp`
    /// upstream. `None` ⇒ MCP isn't configured for this invocation;
    /// every `McpKind::ObjectiveAi` request 501s the same way
    /// `objectiveai_sdk::http::RejectHandler` would.
    mcp_url: Option<String>,
    client: objectiveai_sdk::mcp::Client,
    /// Every dialed upstream — primary + plugin — keyed by its
    /// native `Mcp-Session-Id`. One entry per CLI-hosted MCP
    /// session. Survives across response_id boundaries; cache miss
    /// for [`McpKind::ObjectiveAi`] re-dials, cache miss for
    /// [`McpKind::Other`] returns `-32001`.
    connections: DashMap<String, Arc<ConduitState>>,
    /// Late-bound: filled by [`ConduitMcpHandler::install_notifier`]
    /// after the WS-creating call returns the notifier. Pump
    /// closures read it at fire time.
    notifier: OnceLock<Notifier>,
    /// Filesystem root for resolving installed plugin manifests at
    /// `dial_plugin_upstream` time. `None` means filesystem is
    /// unavailable.
    config_base_dir: Option<PathBuf>,
    /// `response_id → sibling group` map. Populated at every dial
    /// site by inserting one entry per id in the dial's
    /// `X-OBJECTIVEAI-RESPONSE-IDS` header, all pointing at the same
    /// `Arc<Vec<String>>`. Consumed by
    /// [`ConduitMcpHandler::select_response_ids`]: when a streamed
    /// chunk yields a response_id, its sibling group is looked up
    /// and the losers are evicted. Entries are removed as their
    /// groups are processed so a re-fire on the same winner is a
    /// no-op.
    response_id_groups: DashMap<String, Arc<Vec<String>>>,
}

impl ConduitMcpHandler {
    /// Construct a handler that dials the given URL on first use.
    /// `mcp_url = None` makes every `McpKind::ObjectiveAi` request
    /// reject with 501. `config_base_dir` is the filesystem root the
    /// CLI consults for plugin manifests during plugin-MCP dial.
    pub fn new(mcp_url: Option<String>, config_base_dir: Option<PathBuf>) -> Self {
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
                notifier: OnceLock::new(),
                config_base_dir,
                response_id_groups: DashMap::new(),
            }),
        }
    }

    /// Install the `Notifier` the list-changed pump uses to push
    /// `McpListChanged` frames up the WS. Idempotent — first set
    /// wins; later calls are no-ops. Call once, after
    /// `send_streaming_ws` returns the notifier and before the proxy
    /// could plausibly have triggered upstream `list_changed` fires.
    pub fn install_notifier(&self, notifier: Notifier) {
        let _ = self.inner.notifier.set(notifier);
    }

    /// For each `winner` response_id, look up its sibling group in
    /// [`Inner::response_id_groups`]. Drop every `ConduitState`
    /// whose `response_id` is in `siblings − {winner}`. Subsequent
    /// calls with the same winner are no-ops because we also forget
    /// the losers' entries from the group map.
    pub fn select_response_ids(&self, winners: &std::collections::HashSet<String>) {
        for winner in winners {
            let Some(group_arc) = self.inner.response_id_groups.get(winner) else {
                continue;
            };
            let losers: std::collections::HashSet<String> = group_arc
                .value()
                .iter()
                .filter(|id| id.as_str() != winner.as_str())
                .cloned()
                .collect();
            drop(group_arc);
            if losers.is_empty() {
                continue;
            }
            self.inner
                .connections
                .retain(|_, state| !losers.contains(&state.response_id));
            for loser in &losers {
                self.inner.response_id_groups.remove(loser);
            }
        }
    }
}

impl McpHandler for ConduitMcpHandler {
    async fn handle(&self, request: server_request::Request) -> server_response::Response {
        let id = request.id.clone();
        let mcp_kind = request.mcp_kind.clone();

        let payload = match request.payload {
            server_request::Payload::Initialize(init) => {
                dispatch_initialize(&self.inner, mcp_kind.clone(), init, &request.headers).await
            }
            server_request::Payload::SessionTerminate => {
                dispatch_session_terminate(&self.inner, &request.headers)
            }
            server_request::Payload::ToolsList(params) => {
                match resolve_connection(self, &mcp_kind, &request.headers).await {
                    Ok(state) => dispatch_tools_list(&state, &request.headers, params).await,
                    Err(payload) => payload,
                }
            }
            server_request::Payload::ToolsCall(params) => {
                match resolve_connection(self, &mcp_kind, &request.headers).await {
                    Ok(state) => dispatch_tools_call(&state, &request.headers, params).await,
                    Err(payload) => payload,
                }
            }
            server_request::Payload::ResourcesList(params) => {
                match resolve_connection(self, &mcp_kind, &request.headers).await {
                    Ok(state) => dispatch_resources_list(&state, &request.headers, params).await,
                    Err(payload) => payload,
                }
            }
            server_request::Payload::ResourcesRead(params) => {
                match resolve_connection(self, &mcp_kind, &request.headers).await {
                    Ok(state) => dispatch_resources_read(&state, &request.headers, params).await,
                    Err(payload) => payload,
                }
            }
        };

        server_response::Response {
            id,
            mcp_kind,
            payload,
        }
    }
}

/// Look up the cached upstream by `Mcp-Session-Id` header. On miss
/// for [`McpKind::ObjectiveAi`], re-dial the primary upstream with
/// the inbound session id — the local `objectiveai-mcp` HTTP server
/// is persistent and resumes its session. On miss for
/// [`McpKind::Other`], return `-32001`: the plugin subprocess died
/// with the CLI restart so a fresh `initialize` is the only path
/// forward. The proxy's standard retry logic handles it.
async fn resolve_connection(
    handler: &ConduitMcpHandler,
    mcp_kind: &McpKind,
    headers: &IndexMap<String, String>,
) -> Result<Arc<ConduitState>, server_response::Payload> {
    let Some(session_id) = mcp_session_id_from_headers(headers) else {
        return Err(error_for(
            mcp_kind,
            -32600,
            "missing Mcp-Session-Id header".to_string(),
        ));
    };
    if let Some(existing) = handler.inner.connections.get(&session_id) {
        return Ok(existing.clone());
    }
    // Cache miss. Only the primary can resume across CLI restart.
    if !matches!(mcp_kind, McpKind::ObjectiveAi) {
        return Err(error_for(
            mcp_kind,
            -32001,
            format!("no cached connection for Mcp-Session-Id {session_id:?}"),
        ));
    }
    let Some(mcp_url) = handler.inner.mcp_url.as_ref() else {
        return Err(error_for(
            mcp_kind,
            -32601,
            "this client has no MCP server configured (pass --mcp-address)".to_string(),
        ));
    };
    let agent_instance_hierarchy = agent_instance_hierarchy_from_headers(headers);
    let response_id = response_id_from_headers(headers);
    register_response_id_group(&handler.inner, headers);
    let connect_headers = sanitize_connect_headers(headers);
    let connection = match handler
        .inner
        .client
        .connect(mcp_url.clone(), Some(session_id.clone()), Some(connect_headers))
        .await
    {
        Ok(c) => c,
        Err(e) => {
            return Err(error_for(
                mcp_kind,
                -32603,
                format!("conduit: connect (resume): {e}"),
            ));
        }
    };
    install_list_changed_pump(&connection, handler.inner.clone(), mcp_kind.clone());
    let state = Arc::new(ConduitState {
        connection,
        mcp_kind: mcp_kind.clone(),
        agent_instance_hierarchy,
        response_id,
    });
    handler.inner.connections.insert(session_id, state.clone());
    Ok(state)
}

/// Build a `JsonRpcResult::Err` typed into the corresponding
/// response variant for the inbound request payload. The caller
/// already knows which method failed so we discriminate by the
/// payload that would've been produced on success.
fn error_for(mcp_kind: &McpKind, code: i64, message: String) -> server_response::Payload {
    let _ = mcp_kind;
    // Variant doesn't matter for routing — the wire envelope is
    // already shaped by the caller and pattern-matched by the API
    // via `variant_mismatch` on a misalign. We choose a generic
    // shape (`ToolsList`) since SessionTerminate has no error
    // variant; the API still surfaces code+message+data correctly.
    server_response::Payload::ToolsList(JsonRpcResult::Err {
        code,
        message,
        data: None,
    })
}

// ────────────────────────────────────────────────────────────────
// Per-variant dispatchers
// ────────────────────────────────────────────────────────────────

/// `Initialize`: dispatch on McpKind to dial the right upstream,
/// install the list-changed pump tagged with the McpKind, cache,
/// and return the upstream's verbatim `InitializeResult` plus its
/// native `Mcp-Session-Id`.
async fn dispatch_initialize(
    inner: &Arc<Inner>,
    mcp_kind: McpKind,
    init: server_request::InitializeRequest,
    headers: &IndexMap<String, String>,
) -> server_response::Payload {
    let agent_instance_hierarchy = agent_instance_hierarchy_from_headers(headers);
    let agent_id = agent_id_from_headers(headers);
    let response_id = response_id_from_headers(headers);
    register_response_id_group(inner, headers);
    let stored_session_id = mcp_session_id_from_headers(headers);

    let dial = match &mcp_kind {
        McpKind::ObjectiveAi => {
            let Some(mcp_url) = inner.mcp_url.as_ref() else {
                return server_response::Payload::Initialize(JsonRpcResult::Err {
                    code: -32601,
                    message: "this client has no MCP server configured (pass --mcp-address)"
                        .into(),
                    data: None,
                });
            };
            let connect_headers = sanitize_connect_headers(headers);
            inner
                .client
                .connect(mcp_url.clone(), stored_session_id, Some(connect_headers))
                .await
                .map_err(|e| format!("connect: {e}"))
        }
        McpKind::Other { owner, name, version, mcp } => {
            dial_plugin_upstream(
                inner,
                owner.clone(),
                name.clone(),
                version.clone(),
                mcp.clone(),
                init.args,
                agent_instance_hierarchy.clone(),
                agent_id,
                stored_session_id,
            )
            .await
            .map_err(|e| format!("{e}"))
        }
    };

    let connection = match dial {
        Ok(c) => c,
        Err(message) => {
            return server_response::Payload::Initialize(JsonRpcResult::Err {
                code: -32603,
                message: format!("conduit: {message}"),
                data: None,
            });
        }
    };

    install_list_changed_pump(&connection, inner.clone(), mcp_kind.clone());

    let mcp_session_id = connection.session_id.clone();
    let result = connection.initialize_result.clone();

    inner.connections.insert(
        mcp_session_id.clone(),
        Arc::new(ConduitState {
            connection,
            mcp_kind,
            agent_instance_hierarchy,
            response_id,
        }),
    );

    server_response::Payload::Initialize(JsonRpcResult::Ok {
        result: InitializeReply {
            mcp_session_id,
            result,
        },
    })
}

/// `SessionTerminate`: drop the cached connection by `Mcp-Session-Id`.
/// The SDK's `Connection` Drop tears down the SSE listener and HTTP
/// stream the moment the last `Arc` clone drops; removing from the
/// map is enough.
fn dispatch_session_terminate(
    inner: &Arc<Inner>,
    headers: &IndexMap<String, String>,
) -> server_response::Payload {
    if let Some(session_id) = mcp_session_id_from_headers(headers) {
        inner.connections.remove(&session_id);
    }
    server_response::Payload::SessionTerminate
}

async fn dispatch_tools_list(
    state: &ConduitState,
    headers: &IndexMap<String, String>,
    params: ListToolsRequest,
) -> server_response::Payload {
    let result = upstream_call::<ListToolsRequest, ListToolsResult>(
        &state.connection,
        headers,
        "tools/list",
        &params,
    )
    .await;
    server_response::Payload::ToolsList(into_rpc_result(result))
}

async fn dispatch_tools_call(
    state: &ConduitState,
    headers: &IndexMap<String, String>,
    params: CallToolRequestParams,
) -> server_response::Payload {
    let result = upstream_call::<CallToolRequestParams, CallToolResult>(
        &state.connection,
        headers,
        "tools/call",
        &params,
    )
    .await;
    server_response::Payload::ToolsCall(into_rpc_result(result))
}

async fn dispatch_resources_list(
    state: &ConduitState,
    headers: &IndexMap<String, String>,
    params: ListResourcesRequest,
) -> server_response::Payload {
    let result = upstream_call::<ListResourcesRequest, ListResourcesResult>(
        &state.connection,
        headers,
        "resources/list",
        &params,
    )
    .await;
    server_response::Payload::ResourcesList(into_rpc_result(result))
}

async fn dispatch_resources_read(
    state: &ConduitState,
    headers: &IndexMap<String, String>,
    params: ReadResourceRequestParams,
) -> server_response::Payload {
    let result = upstream_call::<ReadResourceRequestParams, ReadResourceResult>(
        &state.connection,
        headers,
        "resources/read",
        &params,
    )
    .await;
    server_response::Payload::ResourcesRead(into_rpc_result(result))
}

fn into_rpc_result<R>(
    result: Result<JsonRpcResult<R>, ConduitError>,
) -> JsonRpcResult<R> {
    match result {
        Ok(r) => r,
        Err(e) => JsonRpcResult::Err {
            code: -32603,
            message: format!("conduit: {e}"),
            data: None,
        },
    }
}

/// Raw POST through an `mcp::Connection`. Builds a JSON-RPC
/// envelope (`{jsonrpc, id, method, params}`) from the typed
/// `params`, forwards inbound headers verbatim (modulo a hop-by-hop
/// blacklist), sets `Mcp-Session-Id` to the connection's own
/// session id, parses the response body via [`parse_json_or_sse`],
/// and projects the JSON-RPC `{result|error}` shape into the
/// SDK-typed [`JsonRpcResult<R>`].
async fn upstream_call<P, R>(
    conn: &objectiveai_sdk::mcp::Connection,
    headers: &IndexMap<String, String>,
    method: &str,
    params: &P,
) -> Result<JsonRpcResult<R>, ConduitError>
where
    P: serde::Serialize,
    R: serde::de::DeserializeOwned,
{
    let rpc_id = uuid::Uuid::new_v4().to_string();
    let envelope = serde_json::json!({
        "jsonrpc": "2.0",
        "id": rpc_id,
        "method": method,
        "params": params,
    });

    let mut req = conn.http_client.post(&conn.url);
    for (k, v) in headers {
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
        .header("Mcp-Session-Id", &conn.session_id)
        .json(&envelope);

    let resp = req.send().await.map_err(ConduitError::Request)?;
    let resp_text = resp.text().await.map_err(ConduitError::Body)?;
    let Some(body) = parse_json_or_sse(&resp_text) else {
        return Err(ConduitError::MalformedUpstream(
            "empty or unparseable upstream response".into(),
        ));
    };

    if let Some(result) = body.get("result") {
        let typed: R = serde_json::from_value(result.clone())
            .map_err(|e| ConduitError::MalformedUpstream(format!("decode upstream result: {e}")))?;
        return Ok(JsonRpcResult::Ok { result: typed });
    }
    if let Some(err) = body.get("error") {
        let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or(-32603);
        let message = err
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("upstream returned an error envelope without a message")
            .to_string();
        let data = err.get("data").cloned();
        return Ok(JsonRpcResult::Err {
            code,
            message,
            data,
        });
    }
    Err(ConduitError::MalformedUpstream(
        "upstream response missing both `result` and `error`".into(),
    ))
}

// ────────────────────────────────────────────────────────────────
// Plugin dial
// ────────────────────────────────────────────────────────────────

/// Dial a plugin's MCP upstream: verify the plugin manifest declares
/// the requested `mcp` server name, spawn `<plugin> mcp <mcp> begin
/// [--<arg> [value]]`, capture the first `Mcp { url }` notification
/// from its stdout, dial that URL (resuming with `stored_session_id`
/// when present).
///
/// Owner / version are carried through for diagnostic readability +
/// future versioning; today's filesystem layer looks up plugins by
/// `name` alone. The `mcp` field discriminates which of the plugin
/// manifest's declared MCP servers to spawn.
#[allow(clippy::too_many_arguments)]
async fn dial_plugin_upstream(
    inner: &Arc<Inner>,
    plugin_owner: String,
    plugin_name: String,
    plugin_version: String,
    mcp_name: String,
    args: IndexMap<String, Option<String>>,
    agent_instance_hierarchy: String,
    agent_id: String,
    stored_session_id: Option<String>,
) -> Result<objectiveai_sdk::mcp::Connection, ConduitError> {
    let fail = |reason: String| ConduitError::PluginDialFailed {
        plugin_owner: plugin_owner.clone(),
        plugin_name: plugin_name.clone(),
        plugin_version: plugin_version.clone(),
        mcp_name: mcp_name.clone(),
        reason,
    };

    let Some(base_dir) = inner.config_base_dir.clone() else {
        return Err(fail("filesystem unavailable (no config_base_dir)".into()));
    };
    let fs = crate::filesystem::Client::new(Some(base_dir), None::<String>, None::<String>);

    let Some(plugin) = fs.get_plugin(&plugin_name).await else {
        return Err(fail(format!("plugin {plugin_name:?} not installed")));
    };
    if !plugin
        .manifest
        .mcp_servers
        .iter()
        .any(|s| s.name == mcp_name)
    {
        return Err(fail(format!(
            "plugin {plugin_name:?} manifest does not declare mcp server {mcp_name:?}"
        )));
    }
    let Some(exe) = fs.resolve_plugin(&plugin_name).await else {
        return Err(fail(format!("plugin {plugin_name:?} binary not found")));
    };

    let mut cmd = tokio::process::Command::new(&exe);
    cmd.arg("mcp").arg(&mcp_name).arg("begin");
    for (k, v) in &args {
        cmd.arg(format!("--{k}"));
        if let Some(value) = v {
            cmd.arg(value);
        }
    }
    cmd.env("OBJECTIVEAI_AGENT_INSTANCE_HIERARCHY", &agent_instance_hierarchy);
    if !agent_id.is_empty() {
        cmd.env("OBJECTIVEAI_AGENT_ID", &agent_id);
    }
    let mut child = cmd
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| fail(format!("spawn failed: {e}")))?;

    let stdout = child.stdout.take().expect("stdout was piped");
    let stderr = child.stderr.take().expect("stderr was piped");

    use tokio::io::AsyncBufReadExt;
    let reader = tokio::io::BufReader::new(stdout);
    let mut lines = reader.lines();

    let timeout = std::time::Duration::from_secs(30);
    let begin_result = tokio::time::timeout(timeout, async {
        loop {
            let line = match lines.next_line().await {
                Ok(Some(l)) => l,
                Ok(None) => {
                    return Err::<objectiveai_sdk::cli::plugins::Mcp, String>(
                        "plugin exited without emitting mcp{url}".into(),
                    );
                }
                Err(e) => return Err(format!("plugin stdout read error: {e}")),
            };
            let out = match serde_json::from_str::<PluginOutput>(&line) {
                Ok(o) => o,
                Err(_) => continue,
            };
            match out {
                PluginOutput::Typed(TypedPluginOutput::Mcp(mcp)) => return Ok(mcp),
                PluginOutput::Typed(TypedPluginOutput::Error(err)) => {
                    return Err(format!("plugin emitted error: {}", err.message));
                }
                PluginOutput::Notification(_)
                | PluginOutput::Typed(TypedPluginOutput::Command { .. }) => {}
            }
        }
    })
    .await;

    tokio::spawn(async move {
        let stderr_task = tokio::spawn(forward_stderr(stderr));
        while let Ok(Some(_)) = lines.next_line().await {}
        let _ = stderr_task.await;
        let _ = child.wait().await;
    });

    let mcp = match begin_result {
        Ok(Ok(mcp)) => mcp,
        Ok(Err(message)) => return Err(fail(message)),
        Err(_) => return Err(fail("plugin mcp begin timed out".into())),
    };

    let connection = inner
        .client
        .connect(mcp.url, stored_session_id, None)
        .await
        .map_err(|e| fail(format!("connect: {e}")))?;

    Ok(connection)
}

async fn forward_stderr(mut stderr: tokio::process::ChildStderr) {
    use tokio::io::AsyncReadExt;
    let mut buf = [0u8; 4096];
    loop {
        match stderr.read(&mut buf).await {
            Ok(0) | Err(_) => return,
            Ok(n) => {
                use std::io::Write;
                let _ = std::io::stderr().write_all(&buf[..n]);
            }
        }
    }
}

/// Wire `set_on_{tools,resources}_list_changed` to fire-and-forget
/// notifier sends. Closures read the late-bound `Notifier` from the
/// `Inner`'s `OnceLock` at fire time — events that fire before
/// `install_notifier` is called are dropped silently. Each pump is
/// keyed to a single [`McpKind`]; the emitted [`McpListChanged`]
/// frame carries that kind so the API can route to the matching
/// per-MCP GET-SSE subscriber.
fn install_list_changed_pump(
    connection: &objectiveai_sdk::mcp::Connection,
    inner: Arc<Inner>,
    mcp_kind: McpKind,
) {
    let inner_tools = inner.clone();
    let kind_tools = mcp_kind.clone();
    connection.set_on_tools_list_changed(move || {
        let Some(notifier) = inner_tools.notifier.get().cloned() else {
            return;
        };
        let mcp_kind = kind_tools.clone();
        tokio::spawn(async move {
            let _ = notifier
                .notify_list_changed(McpListChanged {
                    mcp_kind,
                    kind: McpListChangedKind::Tools,
                })
                .await;
        });
    });

    connection.set_on_resources_list_changed(move || {
        let Some(notifier) = inner.notifier.get().cloned() else {
            return;
        };
        let mcp_kind = mcp_kind.clone();
        tokio::spawn(async move {
            let _ = notifier
                .notify_list_changed(McpListChanged {
                    mcp_kind,
                    kind: McpListChangedKind::Resources,
                })
                .await;
        });
    });
}

/// Hop-by-hop and layer-internal headers don't propagate to MCP.
fn sanitize_connect_headers(headers: &IndexMap<String, String>) -> IndexMap<String, String> {
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

// ────────────────────────────────────────────────────────────────
// Header helpers
// ────────────────────────────────────────────────────────────────

fn mcp_session_id_from_headers(headers: &IndexMap<String, String>) -> Option<String> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("Mcp-Session-Id"))
        .map(|(_, v)| v.clone())
}

fn agent_instance_hierarchy_from_headers(headers: &IndexMap<String, String>) -> String {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY"))
        .map(|(_, v)| v.clone())
        .unwrap_or_default()
}

fn agent_id_from_headers(headers: &IndexMap<String, String>) -> String {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("X-OBJECTIVEAI-AGENT-ID"))
        .map(|(_, v)| v.clone())
        .unwrap_or_default()
}

fn response_id_from_headers(headers: &IndexMap<String, String>) -> String {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("X-OBJECTIVEAI-RESPONSE-ID"))
        .map(|(_, v)| v.clone())
        .unwrap_or_default()
}

fn response_ids_group_from_headers(headers: &IndexMap<String, String>) -> Vec<String> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("X-OBJECTIVEAI-RESPONSE-IDS"))
        .map(|(_, v)| v.split('-').map(str::to_owned).collect())
        .unwrap_or_default()
}

fn register_response_id_group(inner: &Arc<Inner>, headers: &IndexMap<String, String>) {
    let group = response_ids_group_from_headers(headers);
    if group.is_empty() {
        return;
    }
    let shared = Arc::new(group);
    for id in shared.iter() {
        inner
            .response_id_groups
            .insert(id.clone(), shared.clone());
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
    #[error("upstream response was malformed: {0}")]
    MalformedUpstream(String),
    #[error("plugin upstream {plugin_owner:?}/{plugin_name:?}@{plugin_version:?}/{mcp_name:?} dial failed: {reason}")]
    PluginDialFailed {
        plugin_owner: String,
        plugin_name: String,
        plugin_version: String,
        mcp_name: String,
        reason: String,
    },
}
