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
//! each distinct `McpKind` is one connection. Connections are created
//! only by `initialize`; the conduit never re-dials out of band, so any
//! cache miss returns `-32001` and lets the proxy retry with a fresh
//! `initialize`.
//!
//! `Notifier` is late-bound: the pump needs one, but the `Notifier`
//! is output of `send_streaming_ws(handler, ...)` and the handler is
//! input. The caller constructs the conduit, threads its clone into
//! `send_streaming_ws`, then calls [`ConduitMcpHandler::install_notifier`]
//! on the original handle once the notifier is in hand. Pump
//! closures read the slot at fire time; events that fire before
//! install are dropped (the window is bounded by a few statements
//! at stream startup).

use dashmap::{DashMap, DashSet};
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
    /// For plugin upstreams (`McpKind::Other`): the abort handle of the
    /// stdout-drain task spawned in [`dial_plugin_upstream`]. `None` for
    /// `McpKind::ObjectiveAi`, which has no subprocess.
    ///
    /// Dropping this `ConduitState` aborts that task (see [`Drop`]),
    /// which drops the `execute` stream it owns, which drops the
    /// `tokio::process::Child` — spawned with `kill_on_drop(true)` via
    /// `objectiveai_sdk::subprocess_reaper::spawn` in
    /// `command::plugins::run::execute` — killing the plugin subprocess.
    /// This kill path depends on that `kill_on_drop(true)` staying set.
    plugin_drain: Option<tokio::task::AbortHandle>,
}

impl Drop for ConduitState {
    /// Kill the plugin subprocess tied to this state, if any. Aborting
    /// the drain task drops the stream that owns the plugin's
    /// `tokio::process::Child`; its `kill_on_drop(true)` then kills the
    /// subprocess. No-op for `McpKind::ObjectiveAi` (no subprocess).
    fn drop(&mut self) {
        if let Some(handle) = &self.plugin_drain {
            handle.abort();
        }
    }
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
    mcp_server: crate::http::mcp_server::McpServerHandle,
    client: objectiveai_sdk::mcp::Client,
    /// Every dialed upstream — primary + plugin — keyed by `(objectiveai
    /// response id → McpKind → connection)`. The outer key is the
    /// `X-OBJECTIVEAI-RESPONSE-ID`; the inner key is the request's
    /// `McpKind` (the primary `objectiveai-mcp`, or a specific plugin
    /// owner/name/version/mcp). The conduit never reads the upstream's
    /// `Mcp-Session-Id` for indexing. Connections are created only by
    /// `initialize`; any cache miss returns `-32001`. Inner maps are
    /// reaped on terminate so the outer map doesn't grow unbounded.
    connections: DashMap<String, DashMap<McpKind, Arc<ConduitState>>>,
    /// Which laboratory each in-flight transfer belongs to: the
    /// chunked transfer ops after Begin carry only a `transfer_id`,
    /// but socket routing needs the laboratory — Begin forwards
    /// record the target here; eof/end/abort forwards drop it.
    transfer_routes: DashMap<String, LabTarget>,
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
    /// Objectiveai `response_id`s we've already registered a per-response
    /// MCP notifier for (in the resident hubs' `mcp_notifiers` map).
    /// Register-once guard; this conduit's entries are removed from the
    /// map when [`Inner`] drops (agent completion ended).
    listener_ids: DashSet<String>,
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
        mcp_server: crate::http::mcp_server::McpServerHandle,
        ctx: crate::context::Context,
        agent_tag: Option<String>,
        backoff_max_elapsed_time_ms: u64,
    ) -> Self {
        let http = reqwest::Client::builder()
            .build()
            .expect("reqwest::Client::build is infallible without rustls toggles");
        // The daemon NEVER bounds its own MCP calls — connect + per-call
        // timeouts are `None` (wait forever); only the API applies
        // timeouts anywhere. `backoff_max_elapsed_time_ms` still caps the
        // RETRY budget (give-up-on-errors, not a per-call deadline). The
        // other exponential-backoff knobs are fixed defaults matching the
        // api/proxy (100ms / 100ms / 0.5 / 1.5 / 1000ms).
        let client = objectiveai_sdk::mcp::Client::new(
            http,
            "objectiveai-cli-stream-conduit".to_string(),
            String::new(),
            String::new(),
            None,
            Duration::from_millis(100),
            Duration::from_millis(100),
            0.5,
            1.5,
            Duration::from_millis(1000),
            Duration::from_millis(backoff_max_elapsed_time_ms),
            None,
        );
        Self {
            inner: Arc::new(Inner {
                mcp_server,
                client,
                connections: DashMap::new(),
                transfer_routes: DashMap::new(),
                notifier: OnceLock::new(),
                ctx,
                agent_tag,
                listener_ids: DashSet::new(),
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

    /// Spawn a per-response MCP listener the first time an inbound
    /// server request reveals a `response_id`. The id comes from the
    /// `X-OBJECTIVEAI-RESPONSE-ID` header for the MCP-routed variants,
    /// or the `Drop` body. No-op for requests without one (e.g.
    /// `ReadMessageQueue` / `Retrieve` carry none).
    ///
    /// The notifier is checked BEFORE the id is recorded in
    /// `listener_ids`, so a request that lands in the brief window
    /// before [`Self::install_notifier`] doesn't mark the id "seen"
    /// without a listener — a later request for the same id spawns it.
    fn spawn_listener_if_new(&self, request: &server_request::Request) {
        let Some(response_id) =
            response_id_from_headers(&request.headers).or_else(|| {
                match &request.payload {
                    server_request::Payload::Drop(req) => {
                        Some(req.response_id.clone())
                    }
                    _ => None,
                }
            })
        else {
            return;
        };
        let Some(notifier) = self.inner.notifier.get().cloned() else {
            return;
        };
        // Register `(response_id, notifier)` in the resident daemon's
        // in-process map (was `spawn_mcp_listener` binding a per-response
        // socket). The `listener_ids` guard keeps it register-once, and
        // `Inner`'s `Drop` removes these entries when the conduit tears
        // down. Absent resident hubs (not the daemon) → no-op, as the
        // socket bind was best-effort.
        if self.inner.listener_ids.insert(response_id.clone())
            && let Some(hubs) = self.inner.ctx.resident_hubs()
        {
            hubs.mcp_notifiers.insert(response_id, notifier);
        }
    }
}

impl Drop for Inner {
    /// Remove this conduit's registered `response_id -> Notifier` entries
    /// from the resident daemon's map when the conduit tears down (the
    /// agent completion ended, so the notifier's WS is closing). The
    /// former per-response sockets never cleaned up — this is a strict
    /// improvement over unbounded growth.
    fn drop(&mut self) {
        if let Some(hubs) = self.ctx.resident_hubs() {
            for id in self.listener_ids.iter() {
                hubs.mcp_notifiers.remove(id.key());
            }
        }
    }
}

impl McpHandler for ConduitMcpHandler {
    async fn handle(&self, request: server_request::Request) -> server_response::Response {
        // First server request carrying a response id (header for the
        // MCP-routed variants, body for `Drop`) spins up that response's
        // MCP listener socket. Server requests precede the first chunk,
        // so the socket is ready early. Borrow `&request` here — before
        // the `match` below moves `request.payload`.
        self.spawn_listener_if_new(&request);

        let id = request.id.clone();

        // Laboratory traffic never touches podman or local HTTP any
        // more: every payload addressed to a laboratory — the MCP ops
        // (by `mcp_kind`) and the chunked transfer ops (by inline
        // `laboratory_id`, or `transfer_id` via `transfer_routes`) —
        // forwards through the daemon's `laboratories.sock` to
        // whichever CONNECTED manager owns that laboratory, local or
        // remote alike.
        // A cross-host transfer is ORCHESTRATED here (the splice), not
        // forwarded whole - it addresses TWO hosts. Local transfers
        // (same (machine, state) pair) fall through to the plain
        // forward below and run entirely inside the one host.
        if matches!(
            request.payload,
            server_request::Payload::LaboratoryTransfer(_)
        ) {
            let server_request::Payload::LaboratoryTransfer(req) = request.payload
            else {
                unreachable!("matched LaboratoryTransfer above");
            };
            let payload = self
                .dispatch_laboratory_transfer(&request.headers, req)
                .await;
            return server_response::Response { id, payload };
        }

        if let Some(target) = self.laboratory_target(&request.payload) {
            let payload = self
                .dispatch_laboratory_forward(target, &request.headers, request.payload)
                .await;
            return server_response::Response { id, payload };
        }

        let payload = match request.payload {
            server_request::Payload::Initialize { mcp_kind, params } => {
                dispatch_initialize(&self.inner, mcp_kind, params, &request.headers).await
            }
            server_request::Payload::SessionTerminate { mcp_kind } => {
                dispatch_session_terminate(&self.inner, mcp_kind, &request.headers).await
            }
            server_request::Payload::ToolsList { mcp_kind, params } => {
                match resolve_connection(self, &mcp_kind, &request.headers) {
                    Ok(state) => dispatch_tools_list(&state, &request.headers, params).await,
                    Err((code, message)) => server_response::Payload::ToolsList {
                        mcp_kind,
                        result: rpc_err(code, message),
                    },
                }
            }
            server_request::Payload::ToolsCall { mcp_kind, params } => {
                match resolve_connection(self, &mcp_kind, &request.headers) {
                    Ok(state) => dispatch_tools_call(&state, &request.headers, params).await,
                    Err((code, message)) => server_response::Payload::ToolsCall {
                        mcp_kind,
                        result: rpc_err(code, message),
                    },
                }
            }
            server_request::Payload::ResourcesList { mcp_kind, params } => {
                match resolve_connection(self, &mcp_kind, &request.headers) {
                    Ok(state) => dispatch_resources_list(&state, &request.headers, params).await,
                    Err((code, message)) => server_response::Payload::ResourcesList {
                        mcp_kind,
                        result: rpc_err(code, message),
                    },
                }
            }
            server_request::Payload::ResourcesRead { mcp_kind, params } => {
                match resolve_connection(self, &mcp_kind, &request.headers) {
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
            server_request::Payload::Script(req) => {
                dispatch_script(&self.inner, req).await
            }
            server_request::Payload::Drop(req) => dispatch_drop(&self.inner, req),
            // Laboratory-addressed payloads are intercepted above when a
            // route exists; reaching one of these arms means the transfer
            // id maps to no known laboratory (never Begun here, or its
            // route was already closed).
            server_request::Payload::LaboratoryExportBegin(_) => {
                server_response::Payload::LaboratoryExportBegin(rpc_err(
                    -32001,
                    "laboratory not routable".to_string(),
                ))
            }
            server_request::Payload::LaboratoryExportRead(req) => {
                server_response::Payload::LaboratoryExportRead(rpc_err(
                    -32001,
                    format!("unknown transfer '{}'", req.transfer_id),
                ))
            }
            server_request::Payload::LaboratoryExportAbort(_) => {
                // Abort of an unknown transfer is a successful no-op.
                server_response::Payload::LaboratoryExportAbort(JsonRpcResult::Ok {
                    result: server_response::LaboratoryTransferAck {},
                })
            }
            server_request::Payload::LaboratoryImportBegin(_) => {
                server_response::Payload::LaboratoryImportBegin(rpc_err(
                    -32001,
                    "laboratory not routable".to_string(),
                ))
            }
            server_request::Payload::LaboratoryImportWrite(req) => {
                server_response::Payload::LaboratoryImportWrite(rpc_err(
                    -32001,
                    format!("unknown transfer '{}'", req.transfer_id),
                ))
            }
            server_request::Payload::LaboratoryImportEnd(req) => {
                server_response::Payload::LaboratoryImportEnd(rpc_err(
                    -32001,
                    format!("unknown transfer '{}'", req.transfer_id),
                ))
            }
            server_request::Payload::LaboratoryImportAbort(_) => {
                server_response::Payload::LaboratoryImportAbort(JsonRpcResult::Ok {
                    result: server_response::LaboratoryTransferAck {},
                })
            }
            // Enum-totality floor, not a routing path: both transfer
            // forms are intercepted earlier in this fn by payload
            // VARIANT (the cross-host splice explicitly, the local
            // form via `laboratory_target`), independent of what the
            // laboratories are. Future server-laboratory transfers
            // are orchestrated at the PROXY (its match over the
            // Laboratory enum pair) and never sent to a client
            // conduit — so these arms only ever answer a misbehaving
            // peer, with a typed error rather than a panic.
            server_request::Payload::LaboratoryTransfer(_) => {
                server_response::Payload::LaboratoryTransfer(rpc_err(
                    -32603,
                    "laboratory transfer must be intercepted upstream".to_string(),
                ))
            }
            server_request::Payload::LaboratoryLocalTransfer(_) => {
                server_response::Payload::LaboratoryLocalTransfer(rpc_err(
                    -32603,
                    "laboratory local transfer must be intercepted upstream".to_string(),
                ))
            }
        };

        server_response::Response { id, payload }
    }
}

/// Resolve the cached upstream for this request by `(response id,
/// McpKind)`. A connection is only ever created by `dispatch_initialize`;
/// the conduit never re-dials out of band. A miss here means the proxy
/// issued a non-initialize request for a connection the conduit doesn't
/// hold (no prior `initialize`, or it was already terminated) — return
/// `-32001` so the proxy re-initializes. (This can't reconstruct a
/// plugin's `initialize` args anyway, and the primary should always have
/// been initialized first, so re-dialing would only ever paper over a
/// terminate/call race.)
///
/// On failure returns a bare `(code, message)` — the caller builds the
/// `JsonRpcResult::Err` in the response variant matching its request
/// (see [`rpc_err`]).
fn resolve_connection(
    handler: &ConduitMcpHandler,
    mcp_kind: &McpKind,
    headers: &IndexMap<String, String>,
) -> Result<Arc<ConduitState>, (i64, String)> {
    let Some(response_id) = response_id_from_headers(headers) else {
        return Err((-32600, "missing X-OBJECTIVEAI-RESPONSE-ID header".to_string()));
    };
    get_connection(&handler.inner, &response_id, mcp_kind).ok_or_else(|| {
        (
            -32001,
            format!("no cached connection for response id {response_id:?}"),
        )
    })
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
            // ObjectiveAi has no subprocess, so no drain handle.
            inner
                .client
                .connect(mcp_url, None, Some(connect_headers))
                .await
                .map(|c| (c, None))
                .map_err(|e| format!("connect: {e}"))
        }
        McpKind::Plugin { owner, name, version, mcp } => {
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
            .map(|(c, drain)| (c, Some(drain)))
            .map_err(|e| format!("{e}"))
        }
        McpKind::Laboratory { .. } => {
            // Laboratory payloads are intercepted in `handle` and
            // forwarded over the laboratories socket — they never
            // reach this dial.
            Err("laboratory requests route via the daemon".to_string())
        }
    };

    let (connection, plugin_drain) = match dial {
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
            plugin_drain,
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

/// `Drop`: forceful bulk teardown of every upstream connection for one
/// objectiveai response id. Removes the whole response-id bucket from the
/// registry; dropping it drops every `Arc<ConduitState>` under it, which
/// tears down each MCP connection and kills each plugin subprocess (see
/// `ConduitState`'s `Drop`). Idempotent — `dropped` reports whether a
/// bucket was actually present. The id comes from the payload, not the
/// headers, and no transient headers are required. Infallible.
fn dispatch_drop(
    inner: &Arc<Inner>,
    req: server_request::DropRequest,
) -> server_response::Payload {
    // The removed inner map (if any) drops here, dropping every
    // `Arc<ConduitState>` under this response id.
    let dropped = inner.connections.remove(&req.response_id).is_some();
    server_response::Payload::Drop(server_response::DropResult { dropped })
}

impl ConduitMcpHandler {
    /// Which laboratory (if any) a payload is addressed to — the id
    /// plus the exact host pair when the payload carries it
    /// (laboratory ids are only unique per (machine, state); a
    /// pair-less target routes first-match-by-id).
    fn laboratory_target(&self, payload: &server_request::Payload) -> Option<LabTarget> {
        if let Some(McpKind::Laboratory { id, machine, machine_state, agent }) =
            payload.mcp_kind()
        {
            return Some(LabTarget { id, machine, machine_state, agent_seed: agent });
        }
        match payload {
            server_request::Payload::LaboratoryExportBegin(req) => Some(LabTarget {
                id: req.laboratory_id.clone(),
                machine: req.machine.clone(),
                machine_state: req.machine_state.clone(),
                agent_seed: None,
            }),
            server_request::Payload::LaboratoryImportBegin(req) => Some(LabTarget {
                id: req.laboratory_id.clone(),
                machine: req.machine.clone(),
                machine_state: req.machine_state.clone(),
                agent_seed: None,
            }),
            // Both endpoints share one host (equal (machine, state) by
            // construction) - route by the source pair, forward whole.
            server_request::Payload::LaboratoryLocalTransfer(req) => Some(LabTarget {
                id: req.source_id.clone(),
                machine: req.source_machine.clone(),
                machine_state: req.source_machine_state.clone(),
                agent_seed: None,
            }),
            server_request::Payload::LaboratoryExportRead(req) => self
                .inner
                .transfer_routes
                .get(&req.transfer_id)
                .map(|r| r.clone()),
            server_request::Payload::LaboratoryExportAbort(req) => self
                .inner
                .transfer_routes
                .get(&req.transfer_id)
                .map(|r| r.clone()),
            server_request::Payload::LaboratoryImportWrite(req) => self
                .inner
                .transfer_routes
                .get(&req.transfer_id)
                .map(|r| r.clone()),
            server_request::Payload::LaboratoryImportEnd(req) => self
                .inner
                .transfer_routes
                .get(&req.transfer_id)
                .map(|r| r.clone()),
            server_request::Payload::LaboratoryImportAbort(req) => self
                .inner
                .transfer_routes
                .get(&req.transfer_id)
                .map(|r| r.clone()),
            _ => None,
        }
    }

    /// Forward one laboratory-addressed payload over the daemon's
    /// laboratories socket and maintain `transfer_routes` from what
    /// passes through (Begin replies open a route; eof/end/abort
    /// close it).
    async fn dispatch_laboratory_forward(
        &self,
        mut target: LabTarget,
        headers: &IndexMap<String, String>,
        payload: server_request::Payload,
    ) -> server_response::Payload {
        let shape = LabErrorShape::of(&payload);
        // Route bookkeeping BEFORE the await: aborts always drop their
        // route (even if the forward fails, the driver is done with it).
        match &payload {
            server_request::Payload::LaboratoryExportAbort(req) => {
                self.inner.transfer_routes.remove(&req.transfer_id);
            }
            server_request::Payload::LaboratoryImportAbort(req) => {
                self.inner.transfer_routes.remove(&req.transfer_id);
            }
            server_request::Payload::LaboratoryImportEnd(req) => {
                self.inner.transfer_routes.remove(&req.transfer_id);
            }
            _ => {}
        }
        // In-process: forward straight to the connected laboratory's
        // registry entry (was the laboratories.sock `Forward`).
        let Some(hubs) = self.inner.ctx.resident_hubs() else {
            return shape.error(
                -32603,
                "laboratory forward requires the resident daemon".to_string(),
            );
        };
        // Agent-embedded laboratory: the lab need not pre-exist. On
        // the session-opening Initialize, REUSE the derived id from
        // whichever connected host already serves it; otherwise CREATE
        // it on THIS machine's own-state host right now (no mounts —
        // agent laboratories don't support them). Later ops carry the
        // seed too but skip this block: they ride the registry's
        // normal id routing.
        if let Some(seed) = target.agent_seed.take() {
            let is_initialize =
                matches!(payload, server_request::Payload::Initialize { .. });
            if is_initialize
                && (target.machine.is_none() || target.machine_state.is_none())
            {
                match hubs.laboratories.host_for_laboratory(&target.id).await {
                    Some((machine, machine_state)) => {
                        target.machine = Some(machine);
                        target.machine_state = Some(machine_state);
                    }
                    None => {
                        if let Err(e) = crate::command::laboratories::ensure_local_host(
                            &self.inner.ctx,
                        )
                        .await
                        {
                            return shape.error(
                                -32603,
                                format!(
                                    "agent laboratory {}: local host: {e}",
                                    target.id
                                ),
                            );
                        }
                        let machine = objectiveai_sdk::machine::machine_id(
                            self.inner.ctx.filesystem.dir(),
                        );
                        let machine_state =
                            self.inner.ctx.filesystem.state().to_string();
                        let create = objectiveai_sdk::laboratories::daemon::RequestPayload::Create(
                            objectiveai_sdk::laboratories::daemon::CreateRequest {
                                id: target.id.clone(),
                                image: seed.laboratory.image.clone(),
                                mounts: Vec::new(),
                                env: seed.laboratory.env.clone().unwrap_or_default(),
                                cwd: seed
                                    .laboratory
                                    .cwd
                                    .clone()
                                    .unwrap_or_else(|| "/".to_string()),
                                agent_full_id: Some(seed.agent_full_id.clone()),
                            },
                        );
                        use objectiveai_sdk::laboratories::daemon::{
                            JsonRpcResult as LabRpc, ResponsePayload as LabResp,
                        };
                        match hubs
                            .laboratories
                            .forward_to_host(
                                &machine,
                                &machine_state,
                                indexmap::IndexMap::new(),
                                create,
                            )
                            .await
                        {
                            Ok(LabResp::Create(LabRpc::Ok { .. })) => {}
                            // A concurrent Initialize won the create
                            // race — the lab exists, which is all this
                            // needs.
                            Ok(LabResp::Create(LabRpc::Err { message, .. }))
                                if message.contains("already exists") => {}
                            Ok(LabResp::Create(LabRpc::Err { message, .. })) => {
                                return shape.error(
                                    -32603,
                                    format!(
                                        "agent laboratory {}: create: {message}",
                                        target.id
                                    ),
                                );
                            }
                            Ok(_) => {
                                return shape.error(
                                    -32603,
                                    format!(
                                        "agent laboratory {}: host answered create with an unexpected payload",
                                        target.id
                                    ),
                                );
                            }
                            Err(message) => {
                                return shape.error(
                                    -32603,
                                    format!(
                                        "agent laboratory {}: create: {message}",
                                        target.id
                                    ),
                                );
                            }
                        }
                        target.machine = Some(machine);
                        target.machine_state = Some(machine_state);
                    }
                }
            }
        }
        // TRANSLATE at the seam: the reverse channel and the host
        // channel are naive to each other; the conduit is the one
        // place an API-side op becomes a host-side op (and back).
        let Some((host_payload, mcp_kind)) = to_host_payload(payload) else {
            return shape.error(
                -32603,
                "payload is not laboratory-addressed".to_string(),
            );
        };
        let response = match hubs
            .laboratories
            .forward(
                &target.id,
                target.machine.as_deref(),
                target.machine_state.as_deref(),
                headers.clone(),
                host_payload,
            )
            .await
        {
            Ok(response) => response,
            Err(message) => {
                return shape
                    .error(-32603, format!("laboratory {}: {message}", target.id));
            }
        };
        let response = from_host_payload(response, mcp_kind, &shape);
        // Open/close routes from the manager's replies.
        match &response {
            server_response::Payload::LaboratoryExportBegin(JsonRpcResult::Ok { result }) => {
                self.inner
                    .transfer_routes
                    .insert(result.transfer_id.clone(), target);
            }
            server_response::Payload::LaboratoryImportBegin(JsonRpcResult::Ok { result }) => {
                self.inner
                    .transfer_routes
                    .insert(result.transfer_id.clone(), target);
            }
            server_response::Payload::LaboratoryExportRead(JsonRpcResult::Ok { result }) => {
                if result.eof {
                    // The manager already dropped its entry. Same
                    // FULL target only — a same-id transfer on a
                    // different host keeps its route.
                    self.inner.transfer_routes.retain(|_, lab| *lab != target);
                }
            }
            _ => {}
        }
        response
    }

    /// The daemon-side splice for a cross-host client-to-client
    /// transfer: export from the source host, import into the
    /// destination host, exactly ONE chunk in transit - the API never
    /// touches payload bytes, and this daemon never accumulates beyond
    /// the chunk being moved (the `data` strings pass through opaque,
    /// no base64 work). Abort discipline mirrors the proxy's old
    /// splice: an import-side failure aborts the parked export and
    /// vice versa, so neither host leaks a parked transfer.
    async fn dispatch_laboratory_transfer(
        &self,
        headers: &IndexMap<String, String>,
        req: objectiveai_sdk::client_objectiveai_mcp::server_request::LaboratoryTransferRequest,
    ) -> server_response::Payload {
        use objectiveai_sdk::laboratories::daemon::{
            self as labd, RequestPayload as P, ResponsePayload as HR,
        };
        use server_response::Payload as R;
        let err =
            |code: i64, message: String| R::LaboratoryTransfer(rpc_err(code, message));
        let Some(hubs) = self.inner.ctx.resident_hubs() else {
            return err(
                -32603,
                "laboratory transfer requires the resident daemon".to_string(),
            );
        };
        let labs = &hubs.laboratories;
        let source = (
            req.source_id.clone(),
            req.source_machine.clone(),
            req.source_machine_state.clone(),
        );
        let destination = (
            req.destination_id.clone(),
            req.destination_machine.clone(),
            req.destination_machine_state.clone(),
        );
        let forward = |target: &(String, Option<String>, Option<String>), payload: P| {
            let (id, machine, machine_state) = target.clone();
            let headers = headers.clone();
            async move {
                labs.forward(
                    &id,
                    machine.as_deref(),
                    machine_state.as_deref(),
                    headers,
                    payload,
                )
                .await
            }
        };

        // Export begin on the source host.
        let export_id = match forward(
            &source,
            P::ExportBegin(labd::ExportBeginRequest {
                path: req.source_path.clone(),
            }),
        )
        .await
        {
            Ok(HR::ExportBegin(labd::JsonRpcResult::Ok { result })) => {
                result.transfer_id
            }
            Ok(HR::ExportBegin(labd::JsonRpcResult::Err { code, message, .. })) => {
                return err(code, format!("export begin: {message}"));
            }
            Ok(_) => return err(-32603, "export begin: variant mismatch".to_string()),
            Err(message) => return err(-32603, format!("export begin: {message}")),
        };

        // Import begin on the destination host; abort the parked
        // export on failure.
        let import_id = match forward(
            &destination,
            P::ImportBegin(labd::ImportBeginRequest {
                path: req.destination_path.clone(),
            }),
        )
        .await
        {
            Ok(HR::ImportBegin(labd::JsonRpcResult::Ok { result })) => {
                result.transfer_id
            }
            other => {
                let _ = forward(
                    &source,
                    P::ExportAbort(labd::TransferIdRequest {
                        transfer_id: export_id,
                    }),
                )
                .await;
                return match other {
                    Ok(HR::ImportBegin(labd::JsonRpcResult::Err {
                        code,
                        message,
                        ..
                    })) => err(code, format!("import begin: {message}")),
                    Ok(_) => err(-32603, "import begin: variant mismatch".to_string()),
                    Err(message) => err(-32603, format!("import begin: {message}")),
                };
            }
        };

        // The splice: pull one chunk, push it, repeat. `eof: true`
        // means the export side already dropped its entry (the final
        // chunk's data may still be non-empty).
        loop {
            let chunk = match forward(
                &source,
                P::ExportRead(labd::TransferIdRequest {
                    transfer_id: export_id.clone(),
                }),
            )
            .await
            {
                Ok(HR::ExportRead(labd::JsonRpcResult::Ok { result })) => result,
                other => {
                    let _ = forward(
                        &destination,
                        P::ImportAbort(labd::TransferIdRequest {
                            transfer_id: import_id,
                        }),
                    )
                    .await;
                    return match other {
                        Ok(HR::ExportRead(labd::JsonRpcResult::Err {
                            code,
                            message,
                            ..
                        })) => err(code, format!("export read: {message}")),
                        Ok(_) => {
                            err(-32603, "export read: variant mismatch".to_string())
                        }
                        Err(message) => err(-32603, format!("export read: {message}")),
                    };
                }
            };
            let eof = chunk.eof;
            if !chunk.data.is_empty() {
                match forward(
                    &destination,
                    P::ImportWrite(labd::ImportWriteRequest {
                        transfer_id: import_id.clone(),
                        data: chunk.data,
                    }),
                )
                .await
                {
                    Ok(HR::ImportWrite(labd::JsonRpcResult::Ok { .. })) => {}
                    other => {
                        if !eof {
                            let _ = forward(
                                &source,
                                P::ExportAbort(
                                    labd::TransferIdRequest {
                                        transfer_id: export_id,
                                    },
                                ),
                            )
                            .await;
                        }
                        let _ = forward(
                            &destination,
                            P::ImportAbort(labd::TransferIdRequest {
                                transfer_id: import_id,
                            }),
                        )
                        .await;
                        return match other {
                            Ok(HR::ImportWrite(labd::JsonRpcResult::Err {
                                code,
                                message,
                                ..
                            })) => err(code, format!("import write: {message}")),
                            Ok(_) => {
                                err(-32603, "import write: variant mismatch".to_string())
                            }
                            Err(message) => {
                                err(-32603, format!("import write: {message}"))
                            }
                        };
                    }
                }
            }
            if eof {
                break;
            }
        }

        // Close the import and surface the byte total.
        match forward(
            &destination,
            P::ImportEnd(labd::TransferIdRequest {
                transfer_id: import_id,
            }),
        )
        .await
        {
            Ok(HR::ImportEnd(labd::JsonRpcResult::Ok { result })) => {
                R::LaboratoryTransfer(JsonRpcResult::Ok {
                    result: objectiveai_sdk::client_objectiveai_mcp::server_response::LaboratoryTransferResult {
                        bytes: result.bytes,
                    },
                })
            }
            Ok(HR::ImportEnd(labd::JsonRpcResult::Err { code, message, .. })) => {
                err(code, format!("import end: {message}"))
            }
            Ok(_) => err(-32603, "import end: variant mismatch".to_string()),
            Err(message) => err(-32603, format!("import end: {message}")),
        }
    }
}

/// One laboratory-addressed payload's target: the raw id plus the
/// exact host pair when known. Laboratory ids are only unique per
/// (machine, state) — with the pair the registry forward is direct;
/// without it, first-match-by-id (legacy senders).
#[derive(Clone, PartialEq, Eq)]
struct LabTarget {
    id: String,
    machine: Option<String>,
    machine_state: Option<String>,
    /// For agent-embedded laboratories: the create seed from the
    /// McpKind. Consumed by the Initialize-time reuse-or-create in
    /// `dispatch_laboratory_forward`; `None` on every other route.
    agent_seed: Option<objectiveai_sdk::client_objectiveai_mcp::AgentLaboratorySeed>,
}

/// Enough of a request payload's shape to build a same-variant error
/// reply after the payload itself has been moved into the forward.
#[derive(Clone)]
enum LabErrorShape {
    Initialize(McpKind),
    SessionTerminate(McpKind),
    ToolsList(McpKind),
    ToolsCall(McpKind),
    ResourcesList(McpKind),
    ResourcesRead(McpKind),
    Drop,
    ExportBegin,
    ExportRead,
    ExportAbort,
    ImportBegin,
    ImportWrite,
    ImportEnd,
    ImportAbort,
    LocalTransfer,
    Other,
}

impl LabErrorShape {
    fn of(payload: &server_request::Payload) -> Self {
        use server_request::Payload as P;
        match payload {
            P::Initialize { mcp_kind, .. } => Self::Initialize(mcp_kind.clone()),
            P::SessionTerminate { mcp_kind } => Self::SessionTerminate(mcp_kind.clone()),
            P::ToolsList { mcp_kind, .. } => Self::ToolsList(mcp_kind.clone()),
            P::ToolsCall { mcp_kind, .. } => Self::ToolsCall(mcp_kind.clone()),
            P::ResourcesList { mcp_kind, .. } => Self::ResourcesList(mcp_kind.clone()),
            P::ResourcesRead { mcp_kind, .. } => Self::ResourcesRead(mcp_kind.clone()),
            P::Drop(_) => Self::Drop,
            P::LaboratoryExportBegin(_) => Self::ExportBegin,
            P::LaboratoryExportRead(_) => Self::ExportRead,
            P::LaboratoryExportAbort(_) => Self::ExportAbort,
            P::LaboratoryImportBegin(_) => Self::ImportBegin,
            P::LaboratoryImportWrite(_) => Self::ImportWrite,
            P::LaboratoryImportEnd(_) => Self::ImportEnd,
            P::LaboratoryImportAbort(_) => Self::ImportAbort,
            P::LaboratoryLocalTransfer(_) => Self::LocalTransfer,
            _ => Self::Other,
        }
    }

    fn error(self, code: i64, message: String) -> server_response::Payload {
        use server_response::Payload as R;
        match self {
            Self::Initialize(mcp_kind) => R::Initialize {
                mcp_kind,
                result: rpc_err(code, message),
            },
            Self::SessionTerminate(mcp_kind) => R::SessionTerminate {
                mcp_kind,
                result: rpc_err(code, message),
            },
            Self::ToolsList(mcp_kind) => R::ToolsList {
                mcp_kind,
                result: rpc_err(code, message),
            },
            Self::ToolsCall(mcp_kind) => R::ToolsCall {
                mcp_kind,
                result: rpc_err(code, message),
            },
            Self::ResourcesList(mcp_kind) => R::ResourcesList {
                mcp_kind,
                result: rpc_err(code, message),
            },
            Self::ResourcesRead(mcp_kind) => R::ResourcesRead {
                mcp_kind,
                result: rpc_err(code, message),
            },
            Self::Drop => R::Drop(objectiveai_sdk::client_objectiveai_mcp::server_response::DropResult {
                dropped: false,
            }),
            Self::ExportBegin => R::LaboratoryExportBegin(rpc_err(code, message)),
            Self::ExportRead => R::LaboratoryExportRead(rpc_err(code, message)),
            Self::ExportAbort => R::LaboratoryExportAbort(rpc_err(code, message)),
            Self::ImportBegin => R::LaboratoryImportBegin(rpc_err(code, message)),
            Self::ImportWrite => R::LaboratoryImportWrite(rpc_err(code, message)),
            Self::ImportEnd => R::LaboratoryImportEnd(rpc_err(code, message)),
            Self::ImportAbort => R::LaboratoryImportAbort(rpc_err(code, message)),
            Self::LocalTransfer => R::LaboratoryLocalTransfer(rpc_err(code, message)),
            Self::Other => R::Retrieve(rpc_err(code, message)),
        }
    }
}

/// Reverse-channel op → host-channel op. The two vocabularies are
/// naive to each other; this (with [`from_host_payload`]) is the ONE
/// translation seam. Returns `None` for payloads that are not
/// laboratory-addressed (never happens after `laboratory_target`
/// matched — kept total instead of panicking). The second element is
/// what the response translation needs back: the MCP kind for MCP ops.
fn to_host_payload(
    payload: server_request::Payload,
) -> Option<(
    objectiveai_sdk::laboratories::daemon::RequestPayload,
    Option<McpKind>,
)> {
    use objectiveai_sdk::laboratories::daemon::{self as labd, RequestPayload as H};
    use server_request::Payload as P;
    Some(match payload {
        // MCP ops: the host neither needs nor sees `mcp_kind` (it is
        // reverse-channel routing metadata) — captured here and
        // re-attached by `from_host_payload`.
        P::Initialize { mcp_kind, .. } => (H::Initialize, Some(mcp_kind)),
        P::SessionTerminate { mcp_kind } => (H::SessionTerminate, Some(mcp_kind)),
        P::ToolsList { mcp_kind, params } => (H::ToolsList(params), Some(mcp_kind)),
        P::ToolsCall { mcp_kind, params } => (H::ToolsCall(params), Some(mcp_kind)),
        P::ResourcesList { mcp_kind, params } => {
            (H::ResourcesList(params), Some(mcp_kind))
        }
        P::ResourcesRead { mcp_kind, params } => {
            (H::ResourcesRead(params), Some(mcp_kind))
        }
        // Transfer half-ops: machine/machine_state are daemon-side
        // routing (already consumed by `laboratory_target`), and the
        // per-payload laboratory_id is the envelope's job.
        P::LaboratoryExportBegin(req) => (
            H::ExportBegin(labd::ExportBeginRequest { path: req.path }),
            None,
        ),
        P::LaboratoryExportRead(req) => (
            H::ExportRead(labd::TransferIdRequest { transfer_id: req.transfer_id }),
            None,
        ),
        P::LaboratoryExportAbort(req) => (
            H::ExportAbort(labd::TransferIdRequest { transfer_id: req.transfer_id }),
            None,
        ),
        P::LaboratoryImportBegin(req) => (
            H::ImportBegin(labd::ImportBeginRequest { path: req.path }),
            None,
        ),
        P::LaboratoryImportWrite(req) => (
            H::ImportWrite(labd::ImportWriteRequest {
                transfer_id: req.transfer_id,
                data: req.data,
            }),
            None,
        ),
        P::LaboratoryImportEnd(req) => (
            H::ImportEnd(labd::TransferIdRequest { transfer_id: req.transfer_id }),
            None,
        ),
        P::LaboratoryImportAbort(req) => (
            H::ImportAbort(labd::TransferIdRequest { transfer_id: req.transfer_id }),
            None,
        ),
        P::LaboratoryLocalTransfer(req) => (
            H::LocalTransfer(labd::LocalTransferRequest {
                source_id: req.source_id,
                source_path: req.source_path,
                destination_id: req.destination_id,
                destination_path: req.destination_path,
            }),
            None,
        ),
        _ => return None,
    })
}

/// Host-channel reply → reverse-channel reply, re-attaching the MCP
/// kind captured by [`to_host_payload`]. A host reply whose variant
/// demands a kind we did not capture (impossible via the seam) falls
/// back to the request's error shape.
fn from_host_payload(
    response: objectiveai_sdk::laboratories::daemon::ResponsePayload,
    mcp_kind: Option<McpKind>,
    shape: &LabErrorShape,
) -> server_response::Payload {
    use objectiveai_sdk::laboratories::daemon::ResponsePayload as H;
    use server_response::Payload as R;
    fn conv<T>(
        result: objectiveai_sdk::laboratories::daemon::JsonRpcResult<T>,
    ) -> JsonRpcResult<T> {
        match result {
            objectiveai_sdk::laboratories::daemon::JsonRpcResult::Ok { result } => {
                JsonRpcResult::Ok { result }
            }
            objectiveai_sdk::laboratories::daemon::JsonRpcResult::Err {
                code,
                message,
                data,
            } => JsonRpcResult::Err { code, message, data },
        }
    }
    let kind = |mcp_kind: Option<McpKind>| {
        mcp_kind.ok_or_else(|| {
            shape.clone().error(
                -32603,
                "host reply demands an MCP kind the seam never captured".to_string(),
            )
        })
    };
    match response {
        H::Initialize(result) => match kind(mcp_kind) {
            Ok(mcp_kind) => R::Initialize {
                mcp_kind,
                result: match conv(result) {
                    JsonRpcResult::Ok { result } => JsonRpcResult::Ok {
                        result: server_response::InitializeReply {
                            mcp_session_id: result.mcp_session_id,
                            result: result.result,
                        },
                    },
                    JsonRpcResult::Err { code, message, data } => {
                        JsonRpcResult::Err { code, message, data }
                    }
                },
            },
            Err(error) => error,
        },
        H::SessionTerminate(result) => match kind(mcp_kind) {
            Ok(mcp_kind) => R::SessionTerminate { mcp_kind, result: conv(result) },
            Err(error) => error,
        },
        H::ToolsList(result) => match kind(mcp_kind) {
            Ok(mcp_kind) => R::ToolsList { mcp_kind, result: conv(result) },
            Err(error) => error,
        },
        H::ToolsCall(result) => match kind(mcp_kind) {
            Ok(mcp_kind) => R::ToolsCall { mcp_kind, result: conv(result) },
            Err(error) => error,
        },
        H::ResourcesList(result) => match kind(mcp_kind) {
            Ok(mcp_kind) => R::ResourcesList { mcp_kind, result: conv(result) },
            Err(error) => error,
        },
        H::ResourcesRead(result) => match kind(mcp_kind) {
            Ok(mcp_kind) => R::ResourcesRead { mcp_kind, result: conv(result) },
            Err(error) => error,
        },
        H::Drop(result) => R::Drop(server_response::DropResult {
            dropped: result.dropped,
        }),
        // The daemon's own container-lifecycle signal
        // (`LaboratoryRegistry::send_filetree_signal`) — never a
        // reverse-channel op, so a host echoing it into a forwarded
        // op's reply is a protocol bug.
        H::Filetree(_) => shape.clone().error(
            -32603,
            "unexpected filetree watch reply on the reverse channel".to_string(),
        ),
        H::ExportBegin(result) => R::LaboratoryExportBegin(match conv(result) {
            JsonRpcResult::Ok { result } => JsonRpcResult::Ok {
                result: server_response::LaboratoryTransferBeginResult {
                    transfer_id: result.transfer_id,
                },
            },
            JsonRpcResult::Err { code, message, data } => {
                JsonRpcResult::Err { code, message, data }
            }
        }),
        H::ExportRead(result) => R::LaboratoryExportRead(match conv(result) {
            JsonRpcResult::Ok { result } => JsonRpcResult::Ok {
                result: server_response::LaboratoryExportChunk {
                    data: result.data,
                    eof: result.eof,
                },
            },
            JsonRpcResult::Err { code, message, data } => {
                JsonRpcResult::Err { code, message, data }
            }
        }),
        H::ExportAbort(result) => R::LaboratoryExportAbort(ack(conv(result))),
        H::ImportBegin(result) => R::LaboratoryImportBegin(match conv(result) {
            JsonRpcResult::Ok { result } => JsonRpcResult::Ok {
                result: server_response::LaboratoryTransferBeginResult {
                    transfer_id: result.transfer_id,
                },
            },
            JsonRpcResult::Err { code, message, data } => {
                JsonRpcResult::Err { code, message, data }
            }
        }),
        H::ImportWrite(result) => R::LaboratoryImportWrite(ack(conv(result))),
        H::ImportEnd(result) => R::LaboratoryImportEnd(match conv(result) {
            JsonRpcResult::Ok { result } => JsonRpcResult::Ok {
                result: server_response::LaboratoryImportEndResult {
                    bytes: result.bytes,
                },
            },
            JsonRpcResult::Err { code, message, data } => {
                JsonRpcResult::Err { code, message, data }
            }
        }),
        H::ImportAbort(result) => R::LaboratoryImportAbort(ack(conv(result))),
        H::LocalTransfer(result) => R::LaboratoryLocalTransfer(match conv(result) {
            JsonRpcResult::Ok { result } => JsonRpcResult::Ok {
                result: server_response::LaboratoryTransferResult {
                    bytes: result.bytes,
                },
            },
            JsonRpcResult::Err { code, message, data } => {
                JsonRpcResult::Err { code, message, data }
            }
        }),
        // Host-level ops (create/delete) never enter through this
        // seam — the laboratories commands drive them directly.
        H::Create(_) | H::Delete(_) => shape.clone().error(
            -32603,
            "unexpected host-level reply through the conduit seam".to_string(),
        ),
    }
}

/// Map a host `TransferAck` result onto the reverse channel's ack.
fn ack(
    result: JsonRpcResult<objectiveai_sdk::laboratories::daemon::TransferAck>,
) -> JsonRpcResult<server_response::LaboratoryTransferAck> {
    match result {
        JsonRpcResult::Ok { .. } => JsonRpcResult::Ok {
            result: server_response::LaboratoryTransferAck {},
        },
        JsonRpcResult::Err { code, message, data } => {
            JsonRpcResult::Err { code, message, data }
        }
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
/// One script-execution error reply.
fn script_err(message: impl Into<String>) -> server_response::Payload {
    server_response::Payload::Script(JsonRpcResult::Err {
        code: -32603,
        message: message.into(),
        data: None,
    })
}

/// Run a SCRIPT agent's code in-process on the embedded runtime — the
/// SAME shared `ctx.python()` the `python` command uses. The FULL
/// conversation rides in as the script's `input` global; the script's
/// output deserializes as the assistant/tool-only messages array. No
/// timeout — the runtime's existing posture (the API side owns any
/// deadline discipline).
///
/// The execution context carries the request's TYPED identity fields
/// (the same application `dial_plugin_upstream` does from the MCP
/// path's transient headers), so anything the script runs via
/// `objectiveai.execute` uses the calling agent's identity — its
/// response id, agent full id, lineage.
async fn dispatch_script(
    inner: &Arc<Inner>,
    req: server_request::ScriptRequest,
) -> server_response::Payload {
    let mut exec_ctx = inner.ctx.clone();
    exec_ctx.config.agent_instance_hierarchy = req.agent_instance_hierarchy.clone();
    exec_ctx.config.agent_id = Some(req.agent_id.clone());
    exec_ctx.config.agent_full_id = Some(req.agent_full_id.clone());
    exec_ctx.config.agent_remote = req.agent_remote.clone();
    exec_ctx.config.response_id = Some(req.response_id.clone());
    exec_ctx.config.response_ids = req.response_ids.clone();
    exec_ctx.reset_api_client();
    let python = match exec_ctx.python().await {
        Ok(python) => python,
        Err(e) => return script_err(format!("python runtime: {e}")),
    };
    let objectiveai_sdk::agent::script::Script::Python { python: code } = &req.script;
    let output: Option<Vec<objectiveai_sdk::agent::script::OutputMessage>> =
        match python.exec_code(&exec_ctx, code, Some(&req.messages)).await {
            Ok(output) => output,
            Err(e) => return script_err(format!("script: {e}")),
        };
    let Some(messages) = output else {
        return script_err(
            "script produced no output — it must output a messages array              (assistant/tool roles only)",
        );
    };
    server_response::Payload::Script(JsonRpcResult::Ok {
        result: server_response::ScriptResult { messages },
    })
}

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
) -> Result<(objectiveai_sdk::mcp::Connection, tokio::task::AbortHandle), ConduitError> {
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

    // The drain task OWNS the `execute` stream, which owns the plugin's
    // `tokio::process::Child` (kill_on_drop=true). Keeping its abort
    // handle is what lets us kill the subprocess later: aborting the
    // task drops the stream → drops the Child → kill_on_drop fires.
    let drain = tokio::spawn(async move {
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
    let drain_handle = drain.abort_handle();

    // Wait forever — the API layer above owns the timeout. On any
    // post-spawn failure, abort the drain task so the just-spawned
    // subprocess isn't orphaned (it would otherwise linger until plugin
    // EOF or CLI exit, since the success path is the only one that hands
    // the abort handle to a `ConduitState`).
    let mcp = match mcp_rx.await {
        Ok(mcp) => mcp,
        Err(_) => {
            drain_handle.abort();
            return Err(fail("plugin exited without emitting mcp{url}".into()));
        }
    };

    // Forward the sanitized inbound headers on the handshake so the
    // plugin's MCP server gets the six X-OBJECTIVEAI-* transient
    // headers (incl. AGENT-INSTANCE-HIERARCHY) on initialize + every
    // later RPC — parity with the McpKind::ObjectiveAi dial.
    let connection = match inner
        .client
        .connect(mcp.url, stored_session_id, Some(connect_headers))
        .await
    {
        Ok(connection) => connection,
        Err(e) => {
            drain_handle.abort();
            return Err(fail(format!("connect: {e}")));
        }
    };

    Ok((connection, drain_handle))
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
