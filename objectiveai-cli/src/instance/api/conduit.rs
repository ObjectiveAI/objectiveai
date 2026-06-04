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
use objectiveai_sdk::cli::command::plugins::run::Mcp as PluginMcp;
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
use std::sync::{Arc, OnceLock};
use std::time::Duration;

struct ConduitState {
    connection: objectiveai_sdk::mcp::Connection,
    /// Which upstream this state addresses. Captured at dial time so
    /// the list-changed pump can stamp it on every
    /// [`McpListChanged`] frame.
    mcp_kind: McpKind,
    /// `X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY` of the request that
    /// dialed this upstream. Carried for wire-shape parity and
    /// diagnostic readability.
    agent_instance_hierarchy: String,
}

#[derive(Clone)]
pub struct ConduitMcpHandler {
    inner: Arc<Inner>,
}

struct Inner {
    /// In-process `objectiveai-mcp` server spawned at the top of
    /// `instance::run`. Each `McpKind::ObjectiveAi` dial awaits the
    /// handle's shared port future and builds
    /// `http://127.0.0.1:{port}` on the fly.
    mcp_server: crate::instance::mcp_server::McpServerHandle,
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
    /// Base [`crate::context::Context`] the conduit clones+mutates
    /// per `dial_plugin_upstream` call to stamp the six transient
    /// header values into [`crate::Config`] before calling
    /// [`crate::command::plugins::run::execute`]. Carries the
    /// filesystem client used to resolve installed plugin binaries.
    ctx: crate::context::Context,
}

impl ConduitMcpHandler {
    /// Construct a handler over the given in-process `objectiveai-mcp`
    /// server. `ctx` is the base [`crate::context::Context`] the
    /// conduit clones+mutates per plugin dial to thread the six
    /// transient header values into [`crate::Config`].
    pub fn new(
        mcp_server: crate::instance::mcp_server::McpServerHandle,
        ctx: crate::context::Context,
    ) -> Self {
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
                mcp_server,
                client,
                connections: DashMap::new(),
                notifier: OnceLock::new(),
                ctx,
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
                dispatch_session_terminate(&self.inner, &request.headers).await
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
    let mcp_url = match objectiveai_mcp_url(&handler.inner).await {
        Ok(u) => u,
        Err(message) => return Err(error_for(mcp_kind, -32603, message)),
    };
    let transient = match require_transient(headers) {
        Ok(t) => t,
        Err(message) => {
            return Err(error_for(
                mcp_kind,
                -32600,
                format!("conduit: {message}"),
            ));
        }
    };
    let connect_headers = sanitize_connect_headers(headers);
    let connection = match handler
        .inner
        .client
        .connect(mcp_url, Some(session_id.clone()), Some(connect_headers))
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
        agent_instance_hierarchy: transient.agent_instance_hierarchy,
    });
    handler.inner.connections.insert(session_id, state.clone());
    Ok(state)
}

/// Await the in-process `objectiveai-mcp` server's bound port and
/// build `http://127.0.0.1:{port}`. The shared `oneshot` resolves
/// once the spawner's `setup` has bound the listener; consumers
/// can `clone().await` it any number of times.
async fn objectiveai_mcp_url(inner: &Arc<Inner>) -> Result<String, String> {
    let port = inner
        .mcp_server
        .port
        .clone()
        .await
        .map_err(|_| "in-process objectiveai-mcp failed to bind".to_string())?;
    Ok(format!("http://127.0.0.1:{port}"))
}

/// Build a `JsonRpcResult::Err` typed into the corresponding
/// response variant for the inbound request payload. The caller
/// already knows which method failed so we discriminate by the
/// payload that would've been produced on success.
fn error_for(mcp_kind: &McpKind, code: i64, message: String) -> server_response::Payload {
    let _ = mcp_kind;
    // Used only from `resolve_connection`'s non-Initialize / non-
    // SessionTerminate paths. The API's `variant_mismatch` check
    // logs a mismatch but still surfaces code/message/data, so any
    // `JsonRpcResult::Err` variant works — picking `ToolsList`
    // arbitrarily.
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
    let transient = match require_transient(headers) {
        Ok(t) => t,
        Err(message) => {
            return server_response::Payload::Initialize(JsonRpcResult::Err {
                code: -32600,
                message: format!("conduit: {message}"),
                data: None,
            });
        }
    };
    let stored_session_id = mcp_session_id_from_headers(headers);

    let dial = match &mcp_kind {
        McpKind::ObjectiveAi => {
            let mcp_url = match objectiveai_mcp_url(inner).await {
                Ok(u) => u,
                Err(message) => {
                    return server_response::Payload::Initialize(JsonRpcResult::Err {
                        code: -32603,
                        message,
                        data: None,
                    });
                }
            };
            let connect_headers = sanitize_connect_headers(headers);
            inner
                .client
                .connect(mcp_url, stored_session_id, Some(connect_headers))
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
                &transient,
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
            agent_instance_hierarchy: transient.agent_instance_hierarchy,
        }),
    );

    server_response::Payload::Initialize(JsonRpcResult::Ok {
        result: InitializeReply {
            mcp_session_id,
            result,
        },
    })
}

/// `SessionTerminate`: forward an explicit HTTP DELETE to the
/// upstream MCP server via `Connection::delete()`; on success drop
/// the cached connection. On failure leave the cache entry intact
/// so the proxy can retry — the SDK's `Connection::delete()` already
/// folds upstream 404/401/403 into `Ok(())`, so the only `Err`
/// paths here are real transport / status failures the caller
/// should know about.
async fn dispatch_session_terminate(
    inner: &Arc<Inner>,
    headers: &IndexMap<String, String>,
) -> server_response::Payload {
    let Some(session_id) = mcp_session_id_from_headers(headers) else {
        // Nothing to terminate.
        return server_response::Payload::SessionTerminate(
            JsonRpcResult::Ok { result: () },
        );
    };
    let Some(state) = inner
        .connections
        .get(&session_id)
        .map(|e| e.value().clone())
    else {
        // Not in cache. Idempotent success — the proxy may have
        // already torn down its half.
        return server_response::Payload::SessionTerminate(
            JsonRpcResult::Ok { result: () },
        );
    };
    match state.connection.delete().await {
        Ok(()) => {
            inner.connections.remove(&session_id);
            server_response::Payload::SessionTerminate(
                JsonRpcResult::Ok { result: () },
            )
        }
        Err(e) => server_response::Payload::SessionTerminate(
            JsonRpcResult::Err {
                code: -32603,
                message: format!("conduit: upstream delete: {e}"),
                data: None,
            },
        ),
    }
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

/// Dial a plugin's MCP upstream: clone the base
/// [`crate::context::Context`] from `inner.ctx`, stamp the six
/// transient header values into `Config`, then call the shared
/// [`crate::command::plugins::run::execute`] with
/// `Request { name: plugin, args: ["mcp", mcp_name, "begin", …], jq: None }`.
/// A background drain task forwards the first
/// [`PluginMcp`](objectiveai_sdk::cli::command::plugins::run::Mcp)
/// item it sees via a `tokio::sync::oneshot`, then discards every
/// subsequent stream item until EOF so the plugin's nested-command
/// demux stays unstuck. The CLI does NOT time the dial out — the
/// API layer above owns the deadline; if the plugin exits without
/// ever emitting an Mcp, the oneshot sender drops and we surface
/// that as a `PluginDialFailed`.
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
    transient: &TransientHeaders,
    stored_session_id: Option<String>,
) -> Result<objectiveai_sdk::mcp::Connection, ConduitError> {
    let fail = |reason: String| ConduitError::PluginDialFailed {
        plugin_owner: plugin_owner.clone(),
        plugin_name: plugin_name.clone(),
        plugin_version: plugin_version.clone(),
        mcp_name: mcp_name.clone(),
        reason,
    };

    // Clone base ctx and stamp the six transient headers into
    // Config. `crate::spawn::apply_config_env` (called inside
    // `command::plugins::run::execute`) projects these onto the
    // plugin subprocess env so the plugin's MCP server can
    // re-stamp them on any outbound calls it makes downstream.
    let mut dial_ctx = inner.ctx.clone();
    dial_ctx.config.agent_instance_hierarchy = transient.agent_instance_hierarchy.clone();
    dial_ctx.config.agent_id = Some(transient.agent_id.clone());
    dial_ctx.config.agent_full_id = Some(transient.agent_full_id.clone());
    dial_ctx.config.agent_remote = Some(transient.agent_remote.clone());
    dial_ctx.config.response_id = Some(transient.response_id.clone());
    dial_ctx.config.response_ids = Some(transient.response_ids.clone());

    // Build argv: `mcp <mcp_name> begin [--<k> [<v>]]…`. Manifest /
    // binary resolution is `command::plugins::run::execute`'s job;
    // it surfaces `Error::PluginNotFound` when the plugin isn't
    // installed.
    let mut argv: Vec<String> = vec!["mcp".to_string(), mcp_name.clone(), "begin".to_string()];
    for (k, v) in &args {
        argv.push(format!("--{k}"));
        if let Some(value) = v {
            argv.push(value.clone());
        }
    }

    let request = objectiveai_sdk::cli::command::plugins::run::Request {
        path_type: objectiveai_sdk::cli::command::plugins::run::Path::PluginsRun,
        name: plugin_name.clone(),
        args: argv,
        jq: None,
    };

    let stream = crate::command::plugins::run::execute(&dial_ctx, request)
        .await
        .map_err(|e| fail(format!("plugin spawn failed: {e}")))?;

    let (mcp_tx, mcp_rx) = tokio::sync::oneshot::channel::<PluginMcp>();

    tokio::spawn(async move {
        use futures::StreamExt;
        use objectiveai_sdk::cli::command::plugins::run::ResponseItem;
        let mut stream = stream;
        let mut mcp_tx = Some(mcp_tx);
        while let Some(item) = stream.next().await {
            if let Ok(ResponseItem::Mcp(mcp)) = item {
                if let Some(tx) = mcp_tx.take() {
                    let _ = tx.send(mcp);
                }
            }
            // Every other variant (Error, Notification, stream Err)
            // and every Mcp after the first is discarded — but we
            // keep reading so the plugin's nested-command demux
            // (which writes back into the plugin's stdin) keeps
            // draining the stream until EOF.
        }
        // Stream EOF: if we never saw an Mcp, `mcp_tx` is dropped
        // here, waking `mcp_rx.await` with `Err(Canceled)`.
    });

    // Wait forever — the API layer above owns the timeout.
    let mcp = mcp_rx
        .await
        .map_err(|_| fail("plugin exited without emitting mcp{url}".into()))?;

    let connection = inner
        .client
        .connect(mcp.url, stored_session_id, None)
        .await
        .map_err(|e| fail(format!("connect: {e}")))?;

    Ok(connection)
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

/// The six session-global transient headers the proxy stamps on
/// every outbound request via `Connection.extra_headers`. All
/// required at `initialize` time — the conduit errors if any is
/// missing.
const REQUIRED_TRANSIENT_HEADERS: [&str; 6] = [
    "X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY",
    "X-OBJECTIVEAI-AGENT-ID",
    "X-OBJECTIVEAI-AGENT-FULL-ID",
    "X-OBJECTIVEAI-AGENT-REMOTE",
    "X-OBJECTIVEAI-RESPONSE-ID",
    "X-OBJECTIVEAI-RESPONSE-IDS",
];

/// Verbatim values of the six required transient headers extracted
/// from one `server_request::Request.headers` map. Order matches
/// [`REQUIRED_TRANSIENT_HEADERS`]. Built by [`require_transient`]; a
/// missing key on any of the six is a hard error returned to the
/// API as a `JsonRpcResult::Err`.
struct TransientHeaders {
    agent_instance_hierarchy: String,
    agent_id: String,
    agent_full_id: String,
    agent_remote: String,
    response_id: String,
    response_ids: String,
}

/// Extract all six required transient headers from `headers`. The
/// first missing key (in [`REQUIRED_TRANSIENT_HEADERS`] order) drives
/// the error message — empty-string values count as missing because
/// the proxy never stamps an empty value for these.
fn require_transient(
    headers: &IndexMap<String, String>,
) -> Result<TransientHeaders, String> {
    let mut values: [Option<String>; 6] = Default::default();
    for (idx, key) in REQUIRED_TRANSIENT_HEADERS.iter().enumerate() {
        let v = headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.clone())
            .filter(|v| !v.is_empty());
        match v {
            Some(v) => values[idx] = Some(v),
            None => return Err(format!("missing required header {key:?}")),
        }
    }
    let [agent_instance_hierarchy, agent_id, agent_full_id, agent_remote, response_id, response_ids] =
        values.map(|o| o.expect("every slot filled before this line"));
    Ok(TransientHeaders {
        agent_instance_hierarchy,
        agent_id,
        agent_full_id,
        agent_remote,
        response_id,
        response_ids,
    })
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
