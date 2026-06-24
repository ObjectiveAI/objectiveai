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
//! Storage is a two-level `connections` map keyed by `(objectiveai
//! response id, McpKind)` — the conduit is naive to the upstream's MCP
//! `Mcp-Session-Id`. Both key components ride on every MCP-routed
//! request frame (`mcp_kind` on the payload, `X-OBJECTIVEAI-RESPONSE-ID`
//! in the envelope headers). Each response id owns its own upstream
//! connection set (no cross-response_id sharing); within a response id,
//! each distinct `McpKind` is one connection. Cache miss for the
//! primary upstream reconstructs by re-dialing fresh; plugin cache miss
//! returns `-32001` and lets the proxy retry with a fresh `initialize`
//! — the plugin's disk state covers server-side resume.
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
    mcp_server: crate::websockets::mcp_server::McpServerHandle,
    client: objectiveai_sdk::mcp::Client,
    /// Every dialed upstream — primary + plugin — keyed by `(objectiveai
    /// response id → McpKind → connection)`. The outer key is the
    /// `X-OBJECTIVEAI-RESPONSE-ID`; the inner key is the request's
    /// `McpKind` (the primary `objectiveai-mcp`, or a specific plugin
    /// owner/name/version/mcp). The conduit never reads the upstream's
    /// `Mcp-Session-Id` for indexing. Cache miss for
    /// [`McpKind::ObjectiveAi`] re-dials fresh, cache miss for
    /// [`McpKind::Other`] returns `-32001`. Inner maps are reaped on
    /// terminate so the outer map doesn't grow unbounded.
    connections: DashMap<String, DashMap<McpKind, Arc<ConduitState>>>,
    /// Late-bound: filled by [`ConduitMcpHandler::install_notifier`]
    /// after the WS-creating call returns the notifier. Pump
    /// closures read it at fire time.
    notifier: OnceLock<Notifier>,
    /// Base [`crate::context::Context`] the conduit clones+mutates
    /// per `dial_plugin_upstream` call to stamp the transient
    /// header values (five required + `AGENT-REMOTE` for remote
    /// agents) into [`crate::Config`] before calling
    /// [`crate::command::plugins::run::execute`]. Carries the
    /// filesystem client used to resolve installed plugin binaries.
    ctx: crate::context::Context,
    /// Tag the spawn resolved against, if any. Threaded into
    /// every `dispatch_read_message_queue` call so
    /// `db::message_queue::read_pending_and_upgrade_tag` can fuse
    /// the tag-group upgrade with the read — atomically flipping
    /// every sibling tag in the same `tag_groups` row to BOUND on
    /// the spawn's hierarchy and committing it alongside the row
    /// selection. `None` for the Direct spawn path (no upgrade
    /// to fire).
    agent_tag: Option<String>,
}

impl ConduitMcpHandler {
    /// Construct a handler over the given in-process `objectiveai-mcp`
    /// server. `ctx` is the base [`crate::context::Context`] the
    /// conduit clones+mutates per plugin dial to thread the
    /// transient header values into [`crate::Config`]. `agent_tag`
    /// is the tag the spawn resolved against (if any); when present,
    /// each `dispatch_read_message_queue` call fuses the tag-group
    /// upgrade with the row read in one transaction.
    pub fn new(
        mcp_server: crate::websockets::mcp_server::McpServerHandle,
        ctx: crate::context::Context,
        agent_tag: Option<String>,
        mcp_timeout_ms: u64,
        backoff_max_elapsed_time_ms: u64,
    ) -> Self {
        let http = reqwest::Client::builder()
            .build()
            .expect("reqwest::Client::build is infallible without rustls toggles");
        // The CLI's single MCP client uses the two configured values:
        // `mcp_timeout_ms` for BOTH the connect + per-call timeout, and
        // `backoff_max_elapsed_time_ms` for the retry budget. The other
        // exponential-backoff knobs are fixed defaults matching the
        // api/proxy (100ms / 100ms / 0.5 / 1.5 / 1000ms).
        let client = objectiveai_sdk::mcp::Client::new(
            http,
            "objectiveai-cli-stream-conduit".to_string(),
            String::new(),
            String::new(),
            Duration::from_millis(mcp_timeout_ms),
            Duration::from_millis(100),
            Duration::from_millis(100),
            0.5,
            1.5,
            Duration::from_millis(1000),
            Duration::from_millis(backoff_max_elapsed_time_ms),
            Duration::from_millis(mcp_timeout_ms),
        );
        Self {
            inner: Arc::new(Inner {
                mcp_server,
                client,
                connections: DashMap::new(),
                notifier: OnceLock::new(),
                ctx,
                agent_tag,
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

        let payload = match request.payload {
            server_request::Payload::Initialize { mcp_kind, params } => {
                dispatch_initialize(&self.inner, mcp_kind, params, &request.headers).await
            }
            server_request::Payload::SessionTerminate { mcp_kind } => {
                dispatch_session_terminate(&self.inner, mcp_kind, &request.headers).await
            }
            server_request::Payload::ToolsList { mcp_kind, params } => {
                match resolve_connection(self, &mcp_kind, &request.headers).await {
                    Ok(state) => dispatch_tools_list(&state, &request.headers, params).await,
                    Err((code, message)) => server_response::Payload::ToolsList {
                        mcp_kind,
                        result: rpc_err(code, message),
                    },
                }
            }
            server_request::Payload::ToolsCall { mcp_kind, params } => {
                match resolve_connection(self, &mcp_kind, &request.headers).await {
                    Ok(state) => dispatch_tools_call(&state, &request.headers, params).await,
                    Err((code, message)) => server_response::Payload::ToolsCall {
                        mcp_kind,
                        result: rpc_err(code, message),
                    },
                }
            }
            server_request::Payload::ResourcesList { mcp_kind, params } => {
                match resolve_connection(self, &mcp_kind, &request.headers).await {
                    Ok(state) => dispatch_resources_list(&state, &request.headers, params).await,
                    Err((code, message)) => server_response::Payload::ResourcesList {
                        mcp_kind,
                        result: rpc_err(code, message),
                    },
                }
            }
            server_request::Payload::ResourcesRead { mcp_kind, params } => {
                match resolve_connection(self, &mcp_kind, &request.headers).await {
                    Ok(state) => dispatch_resources_read(&state, &request.headers, params).await,
                    Err((code, message)) => server_response::Payload::ResourcesRead {
                        mcp_kind,
                        result: rpc_err(code, message),
                    },
                }
            }
            server_request::Payload::ReadMessageQueue(req) => {
                dispatch_read_message_queue(&self.inner, req).await
            }
            server_request::Payload::Retrieve(req) => {
                dispatch_retrieve(&self.inner, req).await
            }
        };

        server_response::Response { id, payload }
    }
}

/// Resolve the cached upstream for this request by `(response id,
/// McpKind)`. On miss for [`McpKind::ObjectiveAi`], re-dial the primary
/// upstream fresh — the local `objectiveai-mcp` HTTP server is
/// persistent and the conduit no longer tracks a session id to resume
/// with. On miss for [`McpKind::Other`], return `-32001`: the plugin
/// subprocess died with the CLI restart so a fresh `initialize` is the
/// only path forward. The proxy's standard retry logic handles it.
///
/// On failure returns a bare `(code, message)` — the caller builds the
/// `JsonRpcResult::Err` in the response variant matching its request
/// (see [`rpc_err`]).
async fn resolve_connection(
    handler: &ConduitMcpHandler,
    mcp_kind: &McpKind,
    headers: &IndexMap<String, String>,
) -> Result<Arc<ConduitState>, (i64, String)> {
    let Some(response_id) = response_id_from_headers(headers) else {
        return Err((-32600, "missing X-OBJECTIVEAI-RESPONSE-ID header".to_string()));
    };
    if let Some(existing) = get_connection(&handler.inner, &response_id, mcp_kind) {
        return Ok(existing);
    }
    // Cache miss. Only the primary can be re-dialed; a plugin's
    // subprocess died with the CLI, so a fresh `initialize` is required.
    if !matches!(mcp_kind, McpKind::ObjectiveAi) {
        return Err((
            -32001,
            format!("no cached connection for response id {response_id:?}"),
        ));
    }
    let mcp_url = match objectiveai_mcp_url(&handler.inner).await {
        Ok(u) => u,
        Err(message) => return Err((-32603, message)),
    };
    let transient = match require_transient(headers) {
        Ok(t) => t,
        Err(message) => {
            return Err((-32600, format!("conduit: {message}")));
        }
    };
    let connect_headers = sanitize_connect_headers(headers);
    let connection = match handler
        .inner
        .client
        .connect(mcp_url, None, Some(connect_headers))
        .await
    {
        Ok(c) => c,
        Err(e) => {
            return Err((-32603, format!("conduit: connect (re-dial): {e}")));
        }
    };
    install_list_changed_pump(&connection, handler.inner.clone(), mcp_kind.clone());
    let state = Arc::new(ConduitState {
        connection,
        mcp_kind: mcp_kind.clone(),
        agent_instance_hierarchy: transient.agent_instance_hierarchy,
    });
    insert_connection(&handler.inner, response_id, mcp_kind.clone(), state.clone());
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

/// A `JsonRpcResult::Err` whose `T` is inferred from the
/// `Payload::<Method> { result }` field it's assigned to. Each `handle`
/// arm builds its error result with this **in its own response
/// variant**, co-located with the request match arm, so the response
/// variant ALWAYS matches the request variant.
///
/// This matters: the API's reverse channel asserts that a response's
/// payload variant matches the request it answered
/// (`server_response`'s `variant_mismatch`). A fixed error variant
/// (e.g. always `ToolsList`) would surface to the agent as a spurious
/// "wrong payload variant: expected tools_call, got tools_list" and
/// MASK the real error (the `code`/`message` here). Discriminating by
/// the request — which the match already does — keeps them in lockstep
/// by construction.
fn rpc_err<T>(code: i64, message: String) -> JsonRpcResult<T> {
    JsonRpcResult::Err {
        code,
        message,
        data: None,
    }
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
    let initialize_err = |code: i64, message: String| server_response::Payload::Initialize {
        mcp_kind: mcp_kind.clone(),
        result: JsonRpcResult::Err {
            code,
            message,
            data: None,
        },
    };
    let transient = match require_transient(headers) {
        Ok(t) => t,
        Err(message) => {
            return initialize_err(-32600, format!("conduit: {message}"));
        }
    };

    let dial = match &mcp_kind {
        McpKind::ObjectiveAi => {
            let mcp_url = match objectiveai_mcp_url(inner).await {
                Ok(u) => u,
                Err(message) => {
                    return initialize_err(-32603, message);
                }
            };
            let connect_headers = sanitize_connect_headers(headers);
            // No session-id resume hint: the conduit keys by
            // (response_id, McpKind) and is naive to Mcp-Session-Id.
            inner
                .client
                .connect(mcp_url, None, Some(connect_headers))
                .await
                .map_err(|e| format!("connect: {e}"))
        }
        McpKind::Other { owner, name, version, mcp } => {
            let connect_headers = sanitize_connect_headers(headers);
            dial_plugin_upstream(
                inner,
                owner.clone(),
                name.clone(),
                version.clone(),
                mcp.clone(),
                init.args,
                &transient,
                connect_headers,
                None,
            )
            .await
            .map_err(|e| format!("{e}"))
        }
    };

    let connection = match dial {
        Ok(c) => c,
        Err(message) => {
            return initialize_err(-32603, format!("conduit: {message}"));
        }
    };

    install_list_changed_pump(&connection, inner.clone(), mcp_kind.clone());

    // The upstream's own `Mcp-Session-Id` — still returned to the proxy
    // (and stamped on the real upstream call), but NOT the registry key.
    let mcp_session_id = connection.session_id.clone();
    let result = connection.initialize_result.clone();

    insert_connection(
        inner,
        transient.response_id.clone(),
        mcp_kind.clone(),
        Arc::new(ConduitState {
            connection,
            mcp_kind: mcp_kind.clone(),
            agent_instance_hierarchy: transient.agent_instance_hierarchy,
        }),
    );

    server_response::Payload::Initialize {
        mcp_kind,
        result: JsonRpcResult::Ok {
            result: InitializeReply {
                mcp_session_id,
                result,
            },
        },
    }
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
    mcp_kind: McpKind,
    headers: &IndexMap<String, String>,
) -> server_response::Payload {
    let ok = || server_response::Payload::SessionTerminate {
        mcp_kind: mcp_kind.clone(),
        result: JsonRpcResult::Ok { result: () },
    };
    let Some(response_id) = response_id_from_headers(headers) else {
        // Nothing to terminate.
        return ok();
    };
    // Clone the Arc out and drop every DashMap guard before awaiting the
    // upstream DELETE — never hold a guard across `.await`.
    let Some(state) = get_connection(inner, &response_id, &mcp_kind) else {
        // Not in cache. Idempotent success — the proxy may have
        // already torn down its half.
        return ok();
    };
    match state.connection.delete().await {
        Ok(()) => {
            if let Some(by_kind) = inner.connections.get(&response_id) {
                by_kind.remove(&mcp_kind);
            }
            // Reap the response-id entry once its last upstream is gone.
            // `remove_if` re-checks emptiness under the outer shard lock,
            // serialized against `entry().or_default()` inserts.
            inner.connections.remove_if(&response_id, |_, by_kind| by_kind.is_empty());
            ok()
        }
        Err(e) => server_response::Payload::SessionTerminate {
            mcp_kind,
            result: JsonRpcResult::Err {
                code: -32603,
                message: format!("conduit: upstream delete: {e}"),
                data: None,
            },
        },
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
    server_response::Payload::ToolsList {
        mcp_kind: state.mcp_kind.clone(),
        result: into_rpc_result(result),
    }
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
    server_response::Payload::ToolsCall {
        mcp_kind: state.mcp_kind.clone(),
        result: into_rpc_result(result),
    }
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
    server_response::Payload::ResourcesList {
        mcp_kind: state.mcp_kind.clone(),
        result: into_rpc_result(result),
    }
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
    server_response::Payload::ResourcesRead {
        mcp_kind: state.mcp_kind.clone(),
        result: into_rpc_result(result),
    }
}

/// Non-destructive read of the local `message_queue` queue. The
/// fused `read_pending_and_upgrade_tag` returns the joined
/// `(rich_content, ids)` payload directly — separator insertion
/// and content-id collection live in the DB layer now. When the
/// conduit was constructed with a tag, the same call flips every
/// sibling tag in the spawn's `tag_groups` row to BOUND on the
/// live `agent_instance_hierarchy` in the same transaction. Row
/// deletion is handled downstream by the LogWriter via the
/// in-band `request_message_ids` signal — there's no separate
/// `ClearMessageQueue` RPC.
async fn dispatch_read_message_queue(
    inner: &Arc<Inner>,
    req: server_request::ReadMessageQueueRequest,
) -> server_response::Payload {
    let pool = match inner.ctx.db_client().await {
        Ok(pool) => pool,
        Err(e) => {
            return server_response::Payload::ReadMessageQueue(JsonRpcResult::Err {
                code: -32603,
                message: format!("conduit: read_message_queue: {e}"),
                data: None,
            });
        }
    };
    match crate::db::message_queue::read_pending_and_upgrade_tag(
        pool,
        inner.agent_tag.as_deref(),
        &req.agent_instance_hierarchy,
    )
    .await
    {
        Ok(result) => server_response::Payload::ReadMessageQueue(JsonRpcResult::Ok { result }),
        Err(e) => server_response::Payload::ReadMessageQueue(JsonRpcResult::Err {
            code: -32603,
            message: format!("conduit: read_message_queue: {e}"),
            data: None,
        }),
    }
}

/// Extracts `(owner, repository, commit)` from a `Client` remote path.
/// Returns `None` for any other variant — the API only forwards
/// `client` remotes to the conduit for resolution.
fn retrieve_client_fields(
    path: &objectiveai_sdk::RemotePath,
) -> Option<(&str, &str, &str)> {
    match path {
        objectiveai_sdk::RemotePath::Client { owner, repository, commit } => {
            Some((owner, repository, commit))
        }
        _ => None,
    }
}

/// A `Retrieve` reply carrying a JSON-RPC error.
fn retrieve_err(message: impl Into<String>) -> server_response::Payload {
    server_response::Payload::Retrieve(JsonRpcResult::Err {
        code: -32603,
        message: message.into(),
        data: None,
    })
}

/// Resolve a `Client` remote from the CLI's own local storage on
/// behalf of the API, which forwarded the request because the remote
/// is `client`. Reads the base definition (or resolves the latest
/// commit) via the filesystem client carried on the conduit's ctx.
async fn dispatch_retrieve(
    inner: &Arc<Inner>,
    req: objectiveai_sdk::client_objectiveai_mcp::retrieve::Request,
) -> server_response::Payload {
    use crate::filesystem::publish::Kind;
    use objectiveai_sdk::client_objectiveai_mcp::retrieve;

    let fs = &inner.ctx.filesystem;
    let response: retrieve::Response = match req {
        retrieve::Request::GetAgent { path } => {
            let Some((owner, repository, commit)) = retrieve_client_fields(&path) else {
                return retrieve_err("expected a client remote path");
            };
            match fs
                .read_json::<objectiveai_sdk::agent::RemoteAgentBaseWithFallbacks>(
                    Kind::Agents,
                    owner,
                    repository,
                    Some(commit),
                )
                .await
            {
                Ok(opt) => retrieve::Response::GetAgent { agent: opt.map(|(v, _)| v) },
                Err(e) => return retrieve_err(format!("conduit: retrieve agent: {e}")),
            }
        }
        retrieve::Request::GetSwarm { path } => {
            let Some((owner, repository, commit)) = retrieve_client_fields(&path) else {
                return retrieve_err("expected a client remote path");
            };
            match fs
                .read_json::<objectiveai_sdk::swarm::RemoteSwarmBase>(
                    Kind::Swarms,
                    owner,
                    repository,
                    Some(commit),
                )
                .await
            {
                Ok(opt) => retrieve::Response::GetSwarm { swarm: opt.map(|(v, _)| v) },
                Err(e) => return retrieve_err(format!("conduit: retrieve swarm: {e}")),
            }
        }
        retrieve::Request::GetFunction { path } => {
            let Some((owner, repository, commit)) = retrieve_client_fields(&path) else {
                return retrieve_err("expected a client remote path");
            };
            match fs
                .read_json::<objectiveai_sdk::functions::FullRemoteFunction>(
                    Kind::Functions,
                    owner,
                    repository,
                    Some(commit),
                )
                .await
            {
                Ok(opt) => retrieve::Response::GetFunction { function: opt.map(|(v, _)| v) },
                Err(e) => return retrieve_err(format!("conduit: retrieve function: {e}")),
            }
        }
        retrieve::Request::GetProfile { path } => {
            let Some((owner, repository, commit)) = retrieve_client_fields(&path) else {
                return retrieve_err("expected a client remote path");
            };
            match fs
                .read_json::<objectiveai_sdk::functions::RemoteProfile>(
                    Kind::Profiles,
                    owner,
                    repository,
                    Some(commit),
                )
                .await
            {
                Ok(opt) => retrieve::Response::GetProfile { profile: opt.map(|(v, _)| v) },
                Err(e) => return retrieve_err(format!("conduit: retrieve profile: {e}")),
            }
        }
        retrieve::Request::ResolveLatest { kind, path } => {
            let kind = match kind {
                retrieve::Kind::Agents => Kind::Agents,
                retrieve::Kind::Swarms => Kind::Swarms,
                retrieve::Kind::Functions => Kind::Functions,
                retrieve::Kind::Profiles => Kind::Profiles,
            };
            match path {
                objectiveai_sdk::RemotePathCommitOptional::Client {
                    owner,
                    repository,
                    commit,
                } => {
                    let resolved = match commit {
                        Some(c) => Some(c),
                        None => fs.resolve_head(kind, &owner, &repository).ok(),
                    };
                    let path = resolved.map(|commit| objectiveai_sdk::RemotePath::Client {
                        owner,
                        repository,
                        commit,
                    });
                    retrieve::Response::ResolveLatest { path }
                }
                _ => return retrieve_err("expected a client remote path"),
            }
        }
    };
    server_response::Payload::Retrieve(JsonRpcResult::Ok { result: response })
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
/// `Request { name: plugin, args: ["mcp", mcp_name, "begin", …], base: default }`.
/// A background drain task forwards the first
/// [`PluginMcp`](objectiveai_sdk::cli::command::plugins::run::Mcp)
/// item it sees via a `tokio::sync::oneshot`, then discards every
/// subsequent stream item until EOF so the plugin's nested-command
/// demux stays unstuck. The CLI does NOT time the dial out — the
/// API layer above owns the deadline; if the plugin exits without
/// ever emitting an Mcp, the oneshot sender drops and we surface
/// that as a `PluginDialFailed`.
///
/// `connect_headers` is the sanitized inbound header set (see
/// [`sanitize_connect_headers`]) forwarded verbatim onto the upstream
/// handshake, so the plugin's MCP server receives the six
/// `X-OBJECTIVEAI-*` transient headers (notably
/// `X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY`) on `initialize` and every
/// later RPC — parity with the `McpKind::ObjectiveAi` dial path. The
/// `transient` struct (the parsed subset) is still stamped into the
/// spawned subprocess's env so it can re-stamp them downstream.
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
    connect_headers: IndexMap<String, String>,
    stored_session_id: Option<String>,
) -> Result<objectiveai_sdk::mcp::Connection, ConduitError> {
    let fail = |reason: String| ConduitError::PluginDialFailed {
        plugin_owner: plugin_owner.clone(),
        plugin_name: plugin_name.clone(),
        plugin_version: plugin_version.clone(),
        mcp_name: mcp_name.clone(),
        reason,
    };

    // Clone base ctx and stamp the transient headers into Config.
    // `crate::spawn::apply_config_env` (called inside
    // `command::plugins::run::execute`) projects these onto the
    // plugin subprocess env so the plugin's MCP server can
    // re-stamp them on any outbound calls it makes downstream.
    // `agent_remote` is `None` for inline agents (the api omits
    // the header entirely; empty-string transients are forbidden),
    // which `apply_config_env` translates into an `env_remove` of
    // `OBJECTIVEAI_AGENT_REMOTE` on the spawned subprocess.
    let mut dial_ctx = inner.ctx.clone();
    dial_ctx.config.agent_instance_hierarchy = transient.agent_instance_hierarchy.clone();
    dial_ctx.config.agent_id = Some(transient.agent_id.clone());
    dial_ctx.config.agent_full_id = Some(transient.agent_full_id.clone());
    dial_ctx.config.agent_remote = transient.agent_remote.clone();
    dial_ctx.config.response_id = Some(transient.response_id.clone());
    dial_ctx.config.response_ids = Some(transient.response_ids.clone());
    // Nested plugin commands run in-process against this ctx; the
    // transient identity must not reuse (or poison) the conduit
    // owner's memoized API client.
    dial_ctx.reset_api_client();

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
        owner: plugin_owner.clone(),
        name: plugin_name.clone(),
        version: plugin_version.clone(),
        args: argv,
        base: Default::default(),
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

    // Forward the sanitized inbound headers on the handshake so the
    // plugin's MCP server gets the six X-OBJECTIVEAI-* transient
    // headers (incl. AGENT-INSTANCE-HIERARCHY) on initialize + every
    // later RPC — parity with the McpKind::ObjectiveAi dial.
    let connection = inner
        .client
        .connect(mcp.url, stored_session_id, Some(connect_headers))
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

/// The objectiveai response id the conduit keys connections by. Every
/// MCP-routed request frame carries it in the envelope headers (the
/// proxy stamps `X-OBJECTIVEAI-RESPONSE-ID` on every request).
fn response_id_from_headers(headers: &IndexMap<String, String>) -> Option<String> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("X-OBJECTIVEAI-RESPONSE-ID"))
        .map(|(_, v)| v.clone())
}

/// Look a connection up in the two-level `(response_id → McpKind →
/// ConduitState)` registry, cloning the `Arc` out so no DashMap guard is
/// held past return (and never across an `.await`).
fn get_connection(
    inner: &Inner,
    response_id: &str,
    mcp_kind: &McpKind,
) -> Option<Arc<ConduitState>> {
    inner
        .connections
        .get(response_id)
        .and_then(|by_kind| by_kind.get(mcp_kind).map(|e| e.value().clone()))
}

/// Insert a connection into the two-level registry, creating the inner
/// `McpKind` map on first use for this response id. Replaces any existing
/// entry for the same `(response_id, McpKind)`.
fn insert_connection(
    inner: &Inner,
    response_id: String,
    mcp_kind: McpKind,
    state: Arc<ConduitState>,
) {
    inner
        .connections
        .entry(response_id)
        .or_default()
        .insert(mcp_kind, state);
}

/// The five required session-global transient headers the proxy
/// stamps on every outbound request via `Connection.extra_headers`.
/// All five must be present and non-empty at `initialize` time —
/// the conduit errors if any is missing or empty. Empty-string
/// values are forbidden everywhere on the wire; the api enforces
/// the same rule on the egress side.
///
/// `X-OBJECTIVEAI-AGENT-REMOTE` is *optional* (the api omits it
/// entirely for inline agents) and is extracted separately by
/// [`require_transient`]; if present it must also be non-empty.
const REQUIRED_TRANSIENT_HEADERS: [&str; 5] = [
    "X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY",
    "X-OBJECTIVEAI-AGENT-ID",
    "X-OBJECTIVEAI-AGENT-FULL-ID",
    "X-OBJECTIVEAI-RESPONSE-ID",
    "X-OBJECTIVEAI-RESPONSE-IDS",
];

/// The single optional transient header. Present iff the agent is
/// remote; carries the JSON-encoded `RemotePath`. Inline agents
/// have no remote provenance and the api omits the header entirely
/// rather than stamping an empty value (empty-string headers are
/// forbidden end-to-end).
const OPTIONAL_AGENT_REMOTE_HEADER: &str = "X-OBJECTIVEAI-AGENT-REMOTE";

/// Verbatim values of the transient headers extracted from one
/// `server_request::Request.headers` map. Built by
/// [`require_transient`]; a missing or empty required key is a
/// hard error returned to the API as a `JsonRpcResult::Err`.
struct TransientHeaders {
    agent_instance_hierarchy: String,
    agent_id: String,
    agent_full_id: String,
    /// `None` for inline agents (header absent); `Some(non-empty)`
    /// for remote agents.
    agent_remote: Option<String>,
    response_id: String,
    response_ids: String,
}

/// Extract all five required transient headers from `headers` plus
/// the optional `AGENT-REMOTE`. The first missing or empty required
/// key (in [`REQUIRED_TRANSIENT_HEADERS`] order) drives the error
/// message. `AGENT-REMOTE` is allowed to be absent; if present it
/// must be non-empty (empty-string transients are forbidden
/// end-to-end).
fn require_transient(
    headers: &IndexMap<String, String>,
) -> Result<TransientHeaders, String> {
    let mut values: [Option<String>; 5] = Default::default();
    for (idx, key) in REQUIRED_TRANSIENT_HEADERS.iter().enumerate() {
        let raw = headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.clone());
        let v = match raw {
            None => return Err(format!("missing required header {key:?}")),
            Some(s) if s.is_empty() => {
                return Err(format!("empty required header {key:?}"));
            }
            Some(s) => s,
        };
        values[idx] = Some(v);
    }
    let agent_remote = match headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(OPTIONAL_AGENT_REMOTE_HEADER))
        .map(|(_, v)| v.clone())
    {
        None => None,
        Some(s) if s.is_empty() => {
            return Err(format!(
                "empty optional header {OPTIONAL_AGENT_REMOTE_HEADER:?} (absent header is fine; empty value is not)"
            ));
        }
        Some(s) => Some(s),
    };
    let [agent_instance_hierarchy, agent_id, agent_full_id, response_id, response_ids] =
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
