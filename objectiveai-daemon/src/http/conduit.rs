//! `ConduitMcpHandler` — true medium for the proxy's per-MCP
//! Streamable HTTP requests. Each request the API forwards over the
//! WS reverse-attach channel carries a typed [`McpKind`]
//! discriminator that names exactly one upstream MCP server. The
//! daemon holds exactly ONE kind of MCP connection itself — the
//! in-process `objectiveai-mcp` ([`McpKind::ObjectiveAi`]). Every
//! other kind lives on a LABORATORY HOST:
//!
//! - [`McpKind::Plugin`] and agent-embedded [`McpKind::Laboratory`]
//!   upstreams are EPHEMERAL containers — the session-opening
//!   `Initialize` becomes one atomic `{Agent,Plugin}EphemeralCreate`
//!   on a UNIFORMLY RANDOM connected host (the load balancer;
//!   the local host is ensured only when NO host is connected),
//!   which builds/starts the container AND opens its single MCP
//!   connection, succeeding only when both did. Later ops resolve
//!   through the per-response [`HostRoutes`] table to the
//!   host-authoritative ephemeral lab id + pinned host pair.
//! - Client laboratories forward by id/pin as before.
//!
//! The conduit forwards verbatim — no tool renaming, no aggregation,
//! no capability synthesis; capabilities, server name, and protocol
//! version all come from the upstream itself.
//!
//! Storage: `connections` maps `X-OBJECTIVEAI-RESPONSE-ID` → the one
//! objectiveai connection (naive to the upstream `Mcp-Session-Id`);
//! `routes` maps `(response id, wire identity)` → host route for
//! everything host-side. Connections/routes are created only by
//! `initialize`; the conduit never re-dials out of band, so any cache
//! miss returns `-32001` and lets the proxy retry with a fresh
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

/// One cached in-process `objectiveai-mcp` connection — the ONLY MCP
/// connection the daemon holds. Plugin and laboratory MCP servers are
/// containers on laboratory hosts; their sessions live host-side and
/// the conduit merely routes to them (see [`HostRoutes`]).
struct ConduitState {
    connection: objectiveai_sdk::mcp::Connection,
    /// Which upstream this state addresses (always
    /// [`McpKind::ObjectiveAi`] today). Captured at dial time so the
    /// list-changed pump can stamp it on every [`McpListChanged`]
    /// frame.
    mcp_kind: McpKind,
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
    /// The daemon's ONLY MCP connections: one in-process
    /// `objectiveai-mcp` connection per objectiveai response id
    /// (`X-OBJECTIVEAI-RESPONSE-ID` → connection). Plugin and
    /// laboratory sessions live HOST-side and route via
    /// [`Inner::routes`]. The conduit never reads the upstream's
    /// `Mcp-Session-Id` for indexing. Connections are created only by
    /// `initialize`; any cache miss returns `-32001`.
    connections: DashMap<String, Arc<ConduitState>>,
    /// Which laboratory each in-flight transfer belongs to: the
    /// chunked transfer ops after Begin carry only a `transfer_id`,
    /// but socket routing needs the laboratory — Begin forwards
    /// record the target here; eof/end/abort forwards drop it.
    transfer_routes: DashMap<String, LabTarget>,
    /// Late-bound: filled by [`ConduitMcpHandler::install_notifier`]
    /// after the WS-creating call returns the notifier. Pump
    /// closures read it at fire time.
    notifier: OnceLock<Notifier>,
    /// The conduit's context pair. `scoped` is the BASE scope the
    /// script/command dispatches derive their per-call scopes from
    /// ([`ScopedContext::for_request`], stamping the transient header
    /// identities — five required + `AGENT-REMOTE` for remote
    /// agents). `global` carries the shared services (hubs, db,
    /// python).
    global: crate::context::GlobalContext,
    scoped: crate::context::ScopedContext,
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
    /// Every HOST-side session THIS reverse connection opened —
    /// client-laboratory MCP sessions AND the ephemeral (agent /
    /// plugin) laboratories it created — in ONE structure serving
    /// both ROUTING (later ops for an ephemeral resolve the wire's
    /// identity to the host-authoritative lab id + pinned host pair)
    /// and the DEATH SWEEP ([`Inner`]'s drop forwards a host
    /// `Drop { response_id }` per remaining entry — an ABRUPT
    /// reverse-channel death never says goodbye, and without this a
    /// leaked session would pin a regular container against its idle
    /// stop, or leave an ephemeral un-evaporated until its channel
    /// died). The whole map is this connection's host-session state;
    /// connection gone ⇒ state gone.
    routes: HostRoutes,
}

/// The wire-side identity of one host-routed upstream within a
/// response — what later ops carry, resolved to a [`HostRoute`].
#[derive(Clone, PartialEq, Eq, Hash)]
enum RouteKey {
    /// A client (regular) laboratory session — recorded for the death
    /// sweep only (later ops carry their own machine pins on the
    /// wire).
    Client {
        id: String,
        machine: Option<String>,
        machine_state: Option<String>,
    },
    /// An agent-embedded EPHEMERAL laboratory. The wire carries the
    /// content-addressed DERIVED id; the route holds the actual
    /// ephemeral lab id (`{derived}-{response_id}`, host-authoritative
    /// from the create reply).
    Agent { derived_id: String },
    /// A plugin EPHEMERAL laboratory, identified on the wire by its
    /// coordinate trio.
    Plugin {
        owner: String,
        name: String,
        version: String,
    },
}

/// Where a host-routed upstream actually lives: the laboratory id the
/// HOST knows it by, on the exact host pair it was created on /
/// resolved to.
#[derive(Clone, PartialEq, Eq, Hash)]
struct HostRoute {
    lab_id: String,
    machine: Option<String>,
    machine_state: Option<String>,
}

/// The conduit's host-session table: `(response_id, RouteKey) →
/// HostRoute`. See [`Inner::routes`].
#[derive(Default)]
struct HostRoutes(DashMap<(String, RouteKey), HostRoute>);

impl HostRoutes {
    /// Record (or overwrite — a re-create replaces) one route.
    fn record(&self, response_id: &str, key: RouteKey, route: HostRoute) {
        self.0.insert((response_id.to_string(), key), route);
    }

    /// Resolve one route, cloning it out (no guard escapes).
    fn resolve(&self, response_id: &str, key: &RouteKey) -> Option<HostRoute> {
        self.0
            .get(&(response_id.to_string(), key.clone()))
            .map(|e| e.value().clone())
    }

    /// Remove one route (graceful session end).
    fn remove(&self, response_id: &str, key: &RouteKey) {
        self.0.remove(&(response_id.to_string(), key.clone()));
    }

    /// Remove and return every route under `response_id` — the bulk
    /// `Drop` teardown. Deduplicated by [`HostRoute`] (two keys never
    /// share a route today, but the host `Drop` is per (lab, host) so
    /// duplicates would only waste a frame).
    fn take_response(&self, response_id: &str) -> Vec<HostRoute> {
        let keys: Vec<(String, RouteKey)> = self
            .0
            .iter()
            .filter(|e| e.key().0 == response_id)
            .map(|e| e.key().clone())
            .collect();
        let mut out: Vec<HostRoute> = Vec::new();
        for key in keys {
            if let Some((_, route)) = self.0.remove(&key) {
                if !out.contains(&route) {
                    out.push(route);
                }
            }
        }
        out
    }

    /// Drain EVERYTHING — the death sweep. Returns
    /// `(response_id, route)` pairs, deduplicated per pair.
    fn drain_all(&self) -> Vec<(String, HostRoute)> {
        let mut out: Vec<(String, HostRoute)> = Vec::new();
        for entry in self.0.iter() {
            let pair = (entry.key().0.clone(), entry.value().clone());
            if !out.contains(&pair) {
                out.push(pair);
            }
        }
        self.0.clear();
        out
    }
}

impl ConduitMcpHandler {
    /// Construct a handler over the given in-process `objectiveai-mcp`
    /// server. `scoped` is the BASE scope the conduit derives a fresh
    /// per-dial scope from (threading the transient header identities);
    /// `global` carries the shared services. `agent_tag` is the tag the
    /// spawn resolved against (if any); when present, each
    /// `dispatch_read_message_queue` call fuses the tag-group upgrade
    /// with the row read in one transaction.
    pub fn new(
        mcp_server: crate::http::mcp_server::McpServerHandle,
        global: crate::context::GlobalContext,
        scoped: crate::context::ScopedContext,
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
                global,
                scoped,
                agent_tag,
                listener_ids: DashSet::new(),
                routes: HostRoutes::default(),
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
            && let Some(hubs) = self.inner.global.resident_hubs()
        {
            hubs.mcp_notifiers.insert(response_id, notifier);
        }
    }

    /// Fulfill a proxy→daemon `Command` request: run the CLI command
    /// in-process — the same `crate::run` re-entry `/execute` and
    /// `plugins run` use — and stream one `Command` frame per event
    /// back over the WS AS ITEMS ARRIVE (never collected). Spawned so
    /// the conduit's dispatch never blocks on a run.
    ///
    /// Scope identity: the REQUIRED `agent_arguments` are applied
    /// exactly like `/execute` (wire plugin claims inside them are
    /// nulled by `from_agent_arguments`), then the REQUIRED `plugin`
    /// coordinates are stamped with the same [`ScopedContext::with_plugin`]
    /// `plugins run` uses. This authenticated conduit channel is the
    /// deliberate exception to "never trust wire plugin identity": the
    /// API asserts the trio from its typed `X-MCP-Plugins` marker, and
    /// the plugin run-gates then apply to nested commands.
    ///
    /// Frame discipline: `Ack` was already returned by `handle`;
    /// stream errors are NON-terminal `Error` frames; `Done` is ALWAYS
    /// the final frame. A WS send failure breaks the pump — dropping
    /// the run stream cancels the command — but `Done` is still
    /// attempted.
    fn dispatch_command(
        &self,
        id: String,
        agent_arguments: objectiveai_sdk::cli::command::AgentArguments,
        plugin: objectiveai_sdk::mcp::server::Plugin,
        request: objectiveai_sdk::cli::command::Request,
    ) {
        use futures::StreamExt;
        use objectiveai_sdk::client_objectiveai_mcp::server_response::CommandFrame;

        /// One frame onto the WS; `false` = sink gone, stop pumping.
        async fn send(notifier: &Notifier, id: &str, frame: CommandFrame) -> bool {
            notifier
                .send_server_response(&server_response::Response {
                    id: id.to_string(),
                    payload: server_response::Payload::Command { frame },
                })
                .await
                .is_ok()
        }

        let inner = Arc::clone(&self.inner);
        tokio::spawn(async move {
            // Resolve the notifier with a bounded retry over the
            // documented pre-`install_notifier` window (see module
            // docs). Unlike the list-changed pumps, a Command exchange
            // must not silently drop its frames — but if the notifier
            // never lands the WS is gone and there is nobody to tell.
            let mut tries = 0u32;
            let notifier = loop {
                if let Some(n) = inner.notifier.get().cloned() {
                    break n;
                }
                tries += 1;
                if tries >= 50 {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            };

            let scoped = crate::executor::apply_agent_arguments(
                &inner.scoped,
                Some(&agent_arguments),
            )
            .await
            .into_owned()
            .with_plugin(plugin.owner, plugin.name, plugin.version);

            // The `--request` front door: serialize the typed request
            // and re-enter `crate::run`, exactly like `/execute`.
            let request_json = match serde_json::to_string(&request) {
                Ok(json) => json,
                Err(e) => {
                    let _ = send(
                        &notifier,
                        &id,
                        CommandFrame::Error {
                            error: format!("serialize command request: {e}"),
                        },
                    )
                    .await;
                    let _ = send(&notifier, &id, CommandFrame::Done).await;
                    return;
                }
            };
            let args = vec![
                "objectiveai".to_string(),
                "--request".to_string(),
                request_json,
            ];
            match crate::run(args, Some((inner.global.clone(), scoped))).await {
                Ok(crate::RunStream::Execute(mut stream)) => {
                    while let Some(item) = stream.next().await {
                        let frame = match item {
                            Ok(item) => CommandFrame::Item { item },
                            Err(e) => CommandFrame::Error {
                                error: e.output_message().to_string(),
                            },
                        };
                        if !send(&notifier, &id, frame).await {
                            break;
                        }
                    }
                }
                Ok(crate::RunStream::ExecuteTransform(mut stream)) => {
                    while let Some(item) = stream.next().await {
                        let frame = match item {
                            // A jq/python transform yields bare JSON
                            // values; `ResponseItem::Python` is the
                            // untagged bare-value variant —
                            // wire-identical passthrough.
                            Ok(value) => CommandFrame::Item {
                                item: objectiveai_sdk::cli::command::ResponseItem::Python(value),
                            },
                            Err(e) => CommandFrame::Error {
                                error: e.output_message().to_string(),
                            },
                        };
                        if !send(&notifier, &id, frame).await {
                            break;
                        }
                    }
                }
                Err(e) => {
                    let _ = send(
                        &notifier,
                        &id,
                        CommandFrame::Error {
                            error: e.output_message().to_string(),
                        },
                    )
                    .await;
                }
            }
            let _ = send(&notifier, &id, CommandFrame::Done).await;
        });
    }
}

impl Drop for Inner {
    /// Remove this conduit's registered `response_id -> Notifier` entries
    /// from the resident daemon's map when the conduit tears down (the
    /// agent completion ended, so the notifier's WS is closing). The
    /// former per-response sockets never cleaned up — this is a strict
    /// improvement over unbounded growth.
    ///
    /// Also forward a host `Drop { response_id }` for every host-side
    /// session still in this connection's route table: an abrupt
    /// reverse-channel death (API crash, network cut) never sent
    /// `SessionTerminate`/`Drop`. For regular laboratories the leaked
    /// session would pin the container against its idle stop; for
    /// EPHEMERAL (agent/plugin) laboratories the host EVAPORATES the
    /// container on the Drop. Fire-and-forget, all forwards spawned
    /// concurrently: a session the host already dropped (e.g. the
    /// daemon↔host channel bounced, whose detach sweep clears
    /// per-channel sessions — and evaporates channel-owned
    /// ephemerals) answers `dropped: false` harmlessly.
    fn drop(&mut self) {
        if let Some(hubs) = self.global.resident_hubs() {
            for id in self.listener_ids.iter() {
                hubs.mcp_notifiers.remove(id.key());
            }
            let registry = hubs.laboratories.clone();
            // `Drop` is sync; the forwards need the runtime. Teardown
            // outside a runtime (process exit) can skip — the host's
            // own shutdown/boot sweep covers that case.
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                for (response_id, route) in self.routes.drain_all() {
                    let registry = registry.clone();
                    handle.spawn(async move {
                        let _ = registry
                            .forward(
                                &route.lab_id,
                                route.machine.as_deref(),
                                route.machine_state.as_deref(),
                                indexmap::IndexMap::new(),
                                objectiveai_sdk::laboratories::daemon::RequestPayload::Drop(
                                    objectiveai_sdk::laboratories::daemon::DropRequest {
                                        response_id,
                                    },
                                ),
                            )
                            .await;
                    });
                }
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

        // EPHEMERAL upstreams (plugin MCP servers + agent-embedded
        // laboratories): the session-opening Initialize IS the create —
        // one atomic host round trip (`{Agent,Plugin}EphemeralCreate`)
        // that builds/starts the container AND opens its single MCP
        // connection, on a RANDOMLY chosen connected host.
        if let server_request::Payload::Initialize { mcp_kind, .. } = &request.payload
            && Self::is_ephemeral_kind(mcp_kind)
        {
            let payload = self
                .dispatch_ephemeral_initialize(mcp_kind.clone(), &request.headers)
                .await;
            return server_response::Response { id, payload };
        }
        // Later ops for an ephemeral upstream resolve the wire's
        // identity (plugin trio / derived agent-lab id) to the
        // host-authoritative ephemeral lab id + pinned host pair
        // recorded at create.
        if let Some(resolved) = self.ephemeral_target(&request.payload, &request.headers)
        {
            let payload = match resolved {
                Ok((key, target)) => {
                    let response_id = response_id_from_headers(&request.headers);
                    let ends_session = matches!(
                        request.payload,
                        server_request::Payload::SessionTerminate { .. }
                    );
                    let payload = self
                        .dispatch_laboratory_forward(
                            target,
                            &request.headers,
                            request.payload,
                        )
                        .await;
                    if ends_session
                        && let Some(response_id) = response_id
                    {
                        self.inner.routes.remove(&response_id, &key);
                    }
                    payload
                }
                Err(payload) => payload,
            };
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
            server_request::Payload::Command {
                agent_arguments,
                plugin,
                request,
            } => {
                // Spawns the pump and returns immediately — the `Ack`
                // frame IS this handler's single return value; the
                // pump emits `Item`/`Error` frames and the
                // ALWAYS-terminal `Done` via
                // `Notifier::send_server_response`, sharing the
                // request's envelope id (the wire's one multi-frame
                // exchange).
                self.dispatch_command(
                    id.clone(),
                    agent_arguments,
                    plugin,
                    request,
                );
                server_response::Payload::Command {
                    frame: server_response::CommandFrame::Ack,
                }
            }
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
    _mcp_kind: &McpKind,
    headers: &IndexMap<String, String>,
) -> Result<Arc<ConduitState>, (i64, String)> {
    let Some(response_id) = response_id_from_headers(headers) else {
        return Err((-32600, "missing X-OBJECTIVEAI-RESPONSE-ID header".to_string()));
    };
    get_connection(&handler.inner, &response_id).ok_or_else(|| {
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

/// `Initialize` for the in-process `objectiveai-mcp` — the daemon's
/// ONLY dial. Plugin and agent-laboratory initializes are intercepted
/// in `handle` as EPHEMERAL creates; client-laboratory initializes
/// forward through `laboratory_target`. Installs the list-changed
/// pump, caches by response id, and returns the upstream's verbatim
/// `InitializeResult` plus its native `Mcp-Session-Id`.
async fn dispatch_initialize(
    inner: &Arc<Inner>,
    mcp_kind: McpKind,
    _init: server_request::InitializeRequest,
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

    // Interception order in `handle` makes any non-ObjectiveAi kind
    // here a routing bug — answer typed, never dial.
    if !matches!(mcp_kind, McpKind::ObjectiveAi) {
        return initialize_err(
            -32603,
            "conduit: non-objectiveai initialize must be intercepted upstream"
                .to_string(),
        );
    }
    let mcp_url = match objectiveai_mcp_url(inner).await {
        Ok(u) => u,
        Err(message) => {
            return initialize_err(-32603, message);
        }
    };
    let connect_headers = sanitize_connect_headers(headers);
    // No session-id resume hint: the conduit keys by response id and
    // is naive to Mcp-Session-Id.
    let connection = match inner
        .client
        .connect(mcp_url, None, Some(connect_headers))
        .await
    {
        Ok(c) => c,
        Err(e) => {
            return initialize_err(-32603, format!("conduit: connect: {e}"));
        }
    };

    install_list_changed_pump(&connection, inner.clone(), mcp_kind.clone());

    // The upstream's own `Mcp-Session-Id` — still returned to the proxy
    // (and stamped on the real upstream call), but NOT the registry key.
    let mcp_session_id = connection.session_id.clone();
    let result = connection.initialize_result.clone();

    inner.connections.insert(
        transient.response_id.clone(),
        Arc::new(ConduitState {
            connection,
            mcp_kind: mcp_kind.clone(),
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
    let Some(state) = get_connection(inner, &response_id) else {
        // Not in cache. Idempotent success — the proxy may have
        // already torn down its half.
        return ok();
    };
    match state.connection.delete().await {
        Ok(()) => {
            inner.connections.remove(&response_id);
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

/// `Drop`: forceful bulk teardown of everything one objectiveai
/// response id holds — the cached in-process `objectiveai-mcp`
/// connection, AND every host-side session/ephemeral this connection
/// routed for it (the losers of an agent race get their containers
/// evaporated promptly instead of waiting for the conduit's death
/// sweep). Host forwards fan out CONCURRENTLY, fire-and-forget.
/// Idempotent — `dropped` reports whether a local connection was
/// actually present. The id comes from the payload, not the headers,
/// and no transient headers are required. Infallible.
fn dispatch_drop(
    inner: &Arc<Inner>,
    req: server_request::DropRequest,
) -> server_response::Payload {
    let dropped = inner.connections.remove(&req.response_id).is_some();
    if let Some(hubs) = inner.global.resident_hubs() {
        for route in inner.routes.take_response(&req.response_id) {
            let registry = hubs.laboratories.clone();
            let response_id = req.response_id.clone();
            tokio::spawn(async move {
                let _ = registry
                    .forward(
                        &route.lab_id,
                        route.machine.as_deref(),
                        route.machine_state.as_deref(),
                        indexmap::IndexMap::new(),
                        objectiveai_sdk::laboratories::daemon::RequestPayload::Drop(
                            objectiveai_sdk::laboratories::daemon::DropRequest {
                                response_id,
                            },
                        ),
                    )
                    .await;
            });
        }
    }
    server_response::Payload::Drop(server_response::DropResult { dropped })
}

impl ConduitMcpHandler {
    /// Whether this kind is an EPHEMERAL upstream — a plugin MCP
    /// server or an agent-embedded laboratory. Both are one-per-
    /// response containers on laboratory hosts, created (and
    /// connected) by the atomic ephemeral-create ops.
    fn is_ephemeral_kind(mcp_kind: &McpKind) -> bool {
        matches!(
            mcp_kind,
            McpKind::Plugin { .. }
                | McpKind::Laboratory { agent: Some(_), .. }
        )
    }

    /// The wire identity of an ephemeral upstream as a [`RouteKey`].
    fn route_key(mcp_kind: &McpKind) -> Option<RouteKey> {
        match mcp_kind {
            McpKind::Plugin {
                owner,
                name,
                version,
            } => Some(RouteKey::Plugin {
                owner: owner.clone(),
                name: name.clone(),
                version: version.clone(),
            }),
            McpKind::Laboratory { id, agent: Some(_), .. } => {
                Some(RouteKey::Agent {
                    derived_id: id.clone(),
                })
            }
            _ => None,
        }
    }

    /// Resolve a LATER op on an ephemeral upstream (its Initialize was
    /// intercepted as the create) to the host-authoritative lab id +
    /// pinned host pair recorded at create. `None` ⇒ not an ephemeral
    /// op; `Some(Err(payload))` ⇒ typed error (missing response id, or
    /// no route — the proxy re-initializes on `-32001`).
    fn ephemeral_target(
        &self,
        payload: &server_request::Payload,
        headers: &IndexMap<String, String>,
    ) -> Option<Result<(RouteKey, LabTarget), server_response::Payload>> {
        let mcp_kind = payload.mcp_kind()?;
        let key = Self::route_key(&mcp_kind)?;
        let shape = LabErrorShape::of(payload);
        let Some(response_id) = response_id_from_headers(headers) else {
            return Some(Err(shape.error(
                -32600,
                "missing X-OBJECTIVEAI-RESPONSE-ID header".to_string(),
            )));
        };
        match self.inner.routes.resolve(&response_id, &key) {
            Some(route) => Some(Ok((
                key,
                LabTarget {
                    id: route.lab_id,
                    machine: route.machine,
                    machine_state: route.machine_state,
                    agent_seed: None,
                },
            ))),
            None => Some(Err(shape.error(
                -32001,
                format!(
                    "no ephemeral laboratory for response id {response_id:?}"
                ),
            ))),
        }
    }

    /// The EPHEMERAL initialize: ONE atomic host round trip that
    /// creates the container (building/pulling its image if absent),
    /// starts it, and opens its single MCP connection with THIS
    /// request's full header set — succeeding only when all of it
    /// did (a failed connect removes the container host-side and
    /// fails this op, which fails the proxy's upstream connect).
    ///
    /// LOAD BALANCING: every individual create picks a UNIFORMLY
    /// RANDOM connected laboratory host. Only when NO host is
    /// connected does the daemon ensure (spawn) its local host —
    /// never when remote hosts exist (latency: the ensure path can
    /// cold-start podman).
    async fn dispatch_ephemeral_initialize(
        &self,
        mcp_kind: McpKind,
        headers: &IndexMap<String, String>,
    ) -> server_response::Payload {
        let initialize_err =
            |code: i64, message: String| server_response::Payload::Initialize {
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
        let Some(hubs) = self.inner.global.resident_hubs() else {
            return initialize_err(
                -32603,
                "ephemeral create requires the resident daemon".to_string(),
            );
        };
        let (machine, machine_state) = match hubs.laboratories.random_host() {
            Some(pair) => pair,
            None => {
                if let Err(e) = crate::command::laboratories::ensure_local_host(
                    &self.inner.global,
                    &self.inner.scoped,
                )
                .await
                {
                    return initialize_err(
                        -32603,
                        format!("ephemeral create: local host: {e}"),
                    );
                }
                (
                    objectiveai_sdk::machine::machine_id(
                        self.inner.scoped.filesystem.dir(),
                    ),
                    self.inner.scoped.filesystem.state().to_string(),
                )
            }
        };
        let (key, create) = match &mcp_kind {
            McpKind::Plugin {
                owner,
                name,
                version,
            } => (
                RouteKey::Plugin {
                    owner: owner.clone(),
                    name: name.clone(),
                    version: version.clone(),
                },
                objectiveai_sdk::laboratories::daemon::RequestPayload::PluginEphemeralCreate(
                    objectiveai_sdk::laboratories::daemon::PluginEphemeralCreateRequest {
                        response_id: transient.response_id.clone(),
                        owner: owner.clone(),
                        name: name.clone(),
                        version: version.clone(),
                    },
                ),
            ),
            McpKind::Laboratory { id, agent: Some(seed), .. } => (
                RouteKey::Agent {
                    derived_id: id.clone(),
                },
                objectiveai_sdk::laboratories::daemon::RequestPayload::AgentEphemeralCreate(
                    objectiveai_sdk::laboratories::daemon::AgentEphemeralCreateRequest {
                        response_id: transient.response_id.clone(),
                        agent_full_id: seed.agent_full_id.clone(),
                        laboratory: seed.laboratory.clone(),
                    },
                ),
            ),
            _ => {
                return initialize_err(
                    -32603,
                    "conduit: not an ephemeral kind".to_string(),
                );
            }
        };
        // FULL headers on the create — they seed the host's MCP
        // connect into the container (the in-container server needs
        // the agent-argument set), unlike the old empty-headers
        // Create.
        use objectiveai_sdk::laboratories::daemon::{
            JsonRpcResult as LabRpc, ResponsePayload as LabResp,
        };
        let created = match hubs
            .laboratories
            .forward_to_host(&machine, &machine_state, headers.clone(), create)
            .await
        {
            Ok(
                LabResp::AgentEphemeralCreate(LabRpc::Ok { result })
                | LabResp::PluginEphemeralCreate(LabRpc::Ok { result }),
            ) => result,
            Ok(
                LabResp::AgentEphemeralCreate(LabRpc::Err { message, .. })
                | LabResp::PluginEphemeralCreate(LabRpc::Err { message, .. }),
            ) => {
                return initialize_err(
                    -32603,
                    format!("ephemeral create: {message}"),
                );
            }
            Ok(_) => {
                return initialize_err(
                    -32603,
                    "ephemeral create: host answered with an unexpected payload"
                        .to_string(),
                );
            }
            Err(message) => {
                return initialize_err(
                    -32603,
                    format!("ephemeral create: {message}"),
                );
            }
        };
        // Route: later ops, the graceful terminate, the bulk Drop and
        // the death sweep all resolve through this. `identify.id` is
        // the HOST-authoritative ephemeral lab id.
        self.inner.routes.record(
            &transient.response_id,
            key,
            HostRoute {
                lab_id: created.identify.id.clone(),
                machine: Some(machine),
                machine_state: Some(machine_state),
            },
        );
        server_response::Payload::Initialize {
            mcp_kind,
            result: JsonRpcResult::Ok {
                result: InitializeReply {
                    mcp_session_id: created.reply.mcp_session_id,
                    result: created.reply.result,
                },
            },
        }
    }

    /// Which laboratory (if any) a payload is addressed to — the id
    /// plus the exact host pair when the payload carries it
    /// (laboratory ids are only unique per (machine, state); a
    /// pair-less target routes first-match-by-id). Ephemeral kinds
    /// never reach this — [`Self::ephemeral_target`] resolves them
    /// first.
    fn laboratory_target(&self, payload: &server_request::Payload) -> Option<LabTarget> {
        if let Some(McpKind::Laboratory { id, machine, machine_state, agent }) =
            payload.mcp_kind()
        {
            debug_assert!(agent.is_none(), "agent kinds resolve via ephemeral_target");
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
        target: LabTarget,
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
        let Some(hubs) = self.inner.global.resident_hubs() else {
            return shape.error(
                -32603,
                "laboratory forward requires the resident daemon".to_string(),
            );
        };
        // A laboratory addressed to (or plausibly living on) THIS
        // daemon's own (machine, state) may have no CONNECTED host
        // simply because the daemon restarted since the lab was
        // created — the host is a leashed daemon child and died with
        // it. Mirror the id-routed commands' best-effort local ensure
        // BEFORE forwarding: an exact local pair ensures; a pair-less
        // id no connected host serves ensures too (it may well be a
        // local lab). Remote pairs keep the registry's own no-host
        // error — this daemon cannot spawn a host elsewhere.
        {
            let local_machine = objectiveai_sdk::machine::machine_id(
                self.inner.scoped.filesystem.dir(),
            );
            let local_state = self.inner.scoped.filesystem.state();
            let addressed_local = target.machine.as_deref()
                == Some(local_machine.as_str())
                && target.machine_state.as_deref() == Some(local_state);
            let pairless_unserved = target.machine.is_none()
                && target.machine_state.is_none()
                && hubs
                    .laboratories
                    .host_for_laboratory(&target.id)
                    .await
                    .is_none();
            if (addressed_local || pairless_unserved)
                && !hubs.laboratories.has_host(&local_machine, local_state)
            {
                if let Err(e) = crate::command::laboratories::ensure_local_host(
                    &self.inner.global,
                    &self.inner.scoped,
                )
                .await
                {
                    return shape.error(
                        -32603,
                        format!("laboratory {}: local host: {e}", target.id),
                    );
                }
            }
        }
        // Session routes: remember every CLIENT-lab Initialize this
        // reverse connection forwards (keyed by response id + the
        // resolved routing triple), and forget on the graceful end —
        // the remainder is what [`Inner`]'s drop cleans up after an
        // abrupt reverse-channel death. Over-approximate on purpose:
        // a failed Initialize still records, and its teardown Drop is
        // a harmless `dropped: false` no-op host-side. (Ephemeral
        // upstreams record their routes at create and remove them in
        // `handle`'s ephemeral branch — the removal below misses
        // their Client-shaped key harmlessly.)
        if let Some(response_id) = response_id_from_headers(headers) {
            let key = RouteKey::Client {
                id: target.id.clone(),
                machine: target.machine.clone(),
                machine_state: target.machine_state.clone(),
            };
            match &payload {
                server_request::Payload::Initialize { .. } => {
                    self.inner.routes.record(
                        &response_id,
                        key,
                        HostRoute {
                            lab_id: target.id.clone(),
                            machine: target.machine.clone(),
                            machine_state: target.machine_state.clone(),
                        },
                    );
                }
                server_request::Payload::SessionTerminate { .. } => {
                    self.inner.routes.remove(&response_id, &key);
                }
                _ => {}
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
        let Some(hubs) = self.inner.global.resident_hubs() else {
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
        // Host-level ops (create/ephemeral-create/delete) never enter
        // through this seam — the laboratories commands drive them
        // directly.
        H::Create(_)
        | H::AgentEphemeralCreate(_)
        | H::PluginEphemeralCreate(_)
        | H::Delete(_) => shape.clone().error(
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
    let pool = match inner.global.db_client().await {
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
        &pool,
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
/// SAME shared `global.python()` the `python` command uses. The FULL
/// conversation rides in as the script's `input` global; the script's
/// output deserializes as the assistant/tool-only messages array. No
/// timeout — the runtime's existing posture (the API side owns any
/// deadline discipline).
///
/// The execution context carries the request's TYPED identity fields
/// (derived from the MCP path's transient headers), so anything the
/// script runs via `objectiveai.execute` uses the calling agent's
/// identity — its response id, agent full id, lineage.
async fn dispatch_script(
    inner: &Arc<Inner>,
    req: server_request::ScriptRequest,
) -> server_response::Payload {
    // A fresh scope carrying the request's typed identity.
    let exec_scope = inner
        .scoped
        .for_request(crate::context::ScopeIdentity {
            agent_instance_hierarchy: req.agent_instance_hierarchy.clone(),
            agent_id: Some(req.agent_id.clone()),
            agent_full_id: Some(req.agent_full_id.clone()),
            agent_remote: req.agent_remote.clone(),
            response_id: Some(req.response_id.clone()),
            response_ids: req.response_ids.clone(),
            // Script agents are not plugins; the trio's single
            // writer is `plugins run`.
            plugin_owner: None,
            plugin_repository: None,
            plugin_version: None,
        })
        .await;
    let python = match inner.global.python().await {
        Ok(python) => python,
        Err(e) => return script_err(format!("python runtime: {e}")),
    };
    let objectiveai_sdk::agent::script::Script::Python { python: code } = &req.script;
    let output: Option<Vec<objectiveai_sdk::agent::script::OutputMessage>> =
        match python
            .exec_code(&inner.global, &exec_scope, code, Some(&req.messages))
            .await
        {
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
/// commit) via the filesystem client carried on the conduit's scope.
async fn dispatch_retrieve(
    inner: &Arc<Inner>,
    req: objectiveai_sdk::client_objectiveai_mcp::retrieve::Request,
) -> server_response::Payload {
    use crate::filesystem::publish::Kind;
    use objectiveai_sdk::client_objectiveai_mcp::retrieve;

    let fs = &inner.scoped.filesystem;
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

/// Look the response id's in-process `objectiveai-mcp` connection up,
/// cloning the `Arc` out so no DashMap guard is held past return (and
/// never across an `.await`).
fn get_connection(inner: &Inner, response_id: &str) -> Option<Arc<ConduitState>> {
    inner
        .connections
        .get(response_id)
        .map(|e| e.value().clone())
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
// The PARSE is the contract: `require_transient` enforces presence +
// non-emptiness of the full transient set on every session-opening
// request, whether or not the conduit consumes each field directly.
#[allow(dead_code)]
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
}
