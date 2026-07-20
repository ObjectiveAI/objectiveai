//! WS reverse-channel transport for CLI-hosted upstreams.
//!
//! When the proxy is embedded in the API (per request), it is handed a
//! [`ReverseChannel`] — the means to speak the `client_objectiveai_mcp`
//! protocol over the request's WebSocket. Upstreams whose URL scheme is
//! `ws` ([`WsUpstream`]) are reached through it instead of over HTTP:
//!
//! - marked `X-MCP-Plugins` → [`McpKind::PluginLaboratory`]
//! - marked `X-MCP-Laboratories` → [`McpKind::Laboratory`] /
//!   [`McpKind::AgentLaboratory`]
//!
//! Direction split (the API owns the WS itself):
//! - **send**: the proxy emits a `server_request::Request` into the
//!   channel's mpsc; the API serializes it onto the shared WS sink.
//! - **recv**: the API's recv loop demuxes incoming frames by type and
//!   hands the proxy-bound ones back via [`ReverseChannel::deliver_response`]
//!   (the 6 MCP `server_response` variants) and
//!   [`ReverseChannel::deliver_client_request`] (`McpListChanged`). The
//!   proxy correlates responses to its own outstanding requests by id.
//!
//! [`Upstream`] is the proxy's per-upstream handle — either an HTTP
//! [`Connection`] or a [`WsUpstream`] — exposing the slice of the
//! `Connection` interface the [`crate::session::Session`] depends on.

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use dashmap::DashMap;
use futures::StreamExt;
use indexmap::IndexMap;
use objectiveai_sdk::client_objectiveai_mcp::{
    McpKind,
    client_request::{self, McpListChangedKind},
    client_response,
    server_request::{self, InitializeRequest, Request as ServerRequest},
    server_response::{
        self, CommandFrame, JsonRpcResult, Response as ServerResponse,
    },
};
use objectiveai_sdk::mcp::resource::{
    ListResourcesRequest, ReadResourceRequestParams, ReadResourceResult, Resource,
};
use objectiveai_sdk::mcp::tool::{
    CallToolRequestParams, CallToolResult, ListToolsRequest, Tool,
};
use objectiveai_sdk::mcp::{Connection, Error as McpError};
use tokio::sync::{RwLock, mpsc, oneshot};

use crate::session::Session;
use crate::session_manager::SessionManager;

/// A list-changed callback (mirrors `Connection::set_on_*_list_changed`).
type ListChangedCb = Arc<dyn Fn() + Send + Sync>;

struct Inner {
    /// proxy → API → WS. The API drains the paired receiver and writes
    /// each request onto the shared WS sink.
    tx: mpsc::UnboundedSender<ServerRequest>,
    /// Outstanding requests awaiting their `server_response`, by id.
    /// There is NO channel-level round-trip budget: each op passes its
    /// own `Option<Duration>` to [`ReverseChannel::request`] — ws-MCP
    /// calls use the per-request `X-MCP-CALL-TIMEOUT` value, connects
    /// use the connect timeout, and laboratory transfers + drops run
    /// timeout-free.
    pending: DashMap<String, oneshot::Sender<ServerResponse>>,
    /// Outstanding MULTI-FRAME `Command` exchanges, by id. Unlike
    /// `pending`'s one-shot entries, a sender here stays parked across
    /// every frame of the exchange until the terminal
    /// `CommandFrame::Done` (or until the consumer dropped its
    /// stream). Ids are minted by [`ReverseChannel::command`] — the
    /// proxy's OWN id space, never a plugin-supplied id.
    command_streams: DashMap<String, mpsc::UnboundedSender<CommandFrame>>,
    /// list-changed callbacks per upstream, keyed by
    /// `(session response id, McpKind)`: `(tools, resources)`. The
    /// response-id half keeps identical kinds from colliding across
    /// swarm slots sharing this one reverse channel; `None` is the
    /// fallback slot for a registration whose connect headers carried
    /// no response id. Fired when a matching
    /// `client_request::McpListChanged` arrives.
    list_changed: DashMap<
        (Option<String>, McpKind),
        (Option<ListChangedCb>, Option<ListChangedCb>),
    >,
    /// Session registry, late-bound by [`ReverseChannel::wire_sessions`]
    /// in `setup` (the channel is built before the proxy's
    /// `SessionManager` exists). Lets inbound `client_request`s
    /// (`ListTools`/`CallTool`/`ListResources`/`ReadResource`) run the
    /// proxy's aggregated MCP ops by `response_id`.
    sessions: OnceLock<Arc<SessionManager>>,
}

/// Cheaply-cloneable handle the proxy uses to speak over the WS.
#[derive(Clone)]
pub struct ReverseChannel(Arc<Inner>);

impl std::fmt::Debug for ReverseChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReverseChannel").finish_non_exhaustive()
    }
}

impl ReverseChannel {
    /// Build a channel. Returns the channel plus the receiver the API
    /// drains (serializing each `server_request` onto the shared WS sink).
    pub fn new() -> (Self, mpsc::UnboundedReceiver<ServerRequest>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let inner = Inner {
            tx,
            pending: DashMap::new(),
            command_streams: DashMap::new(),
            list_changed: DashMap::new(),
            sessions: OnceLock::new(),
        };
        (Self(Arc::new(inner)), rx)
    }

    /// Late-bind the proxy's session registry so inbound MCP-op
    /// `client_request`s can resolve a session by `response_id`. Called
    /// once by `setup` (idempotent — first write wins).
    pub(crate) fn wire_sessions(&self, sessions: Arc<SessionManager>) {
        let _ = self.0.sessions.set(sessions);
    }

    /// Resolve a session for an inbound MCP-op `client_request`. Returns
    /// a `(code, message)` error suitable for a `JsonRpcResult::Err` when
    /// sessions aren't wired or no session exists for `response_id`.
    /// Resolve a session for a client-request MCP op. A response id
    /// whose initial connect is in flight parks here until the connect
    /// finishes (see `SessionManager::get_or_wait`) instead of failing
    /// with `-32001` — an upstream server may call back in while it is
    /// itself being connected.
    async fn lookup_session(
        &self,
        response_id: &str,
    ) -> Result<Arc<Session>, (i64, String)> {
        let sessions = self
            .0
            .sessions
            .get()
            .ok_or((-32603i64, "proxy sessions not wired".to_string()))?;
        sessions
            .get_or_wait(response_id)
            .await
            .ok_or_else(|| (-32001i64, format!("unknown session for response id {response_id:?}")))
    }

    /// Emit a `server_request` and await its matching `server_response`,
    /// bounded by the CALLER-supplied per-op `timeout` — `None` awaits
    /// with no deadline (resolves on reply, errors on channel drop).
    /// ws-MCP ops pass the per-request call timeout, connects pass the
    /// connect timeout, laboratory transfers and drops pass `None`.
    /// `id` is minted here; the API's recv loop routes the reply back
    /// via [`Self::deliver_response`].
    async fn request(
        &self,
        payload: server_request::Payload,
        headers: IndexMap<String, String>,
        timeout: Option<Duration>,
    ) -> Result<ServerResponse, McpError> {
        let id = uuid::Uuid::new_v4().to_string();
        let (resp_tx, resp_rx) = oneshot::channel();
        self.0.pending.insert(id.clone(), resp_tx);
        let request = ServerRequest {
            id: id.clone(),
            headers,
            payload,
        };
        if self.0.tx.send(request).is_err() {
            self.0.pending.remove(&id);
            return Err(transport_error("reverse channel closed before send"));
        }
        let received = match timeout {
            Some(timeout) => match tokio::time::timeout(timeout, resp_rx).await
            {
                Ok(received) => received,
                Err(_) => {
                    self.0.pending.remove(&id);
                    return Err(transport_error(
                        "reverse channel timed out waiting for response",
                    ));
                }
            },
            None => resp_rx.await,
        };
        match received {
            Ok(response) => Ok(response),
            Err(_) => {
                self.0.pending.remove(&id);
                Err(transport_error("reverse channel dropped before response"))
            }
        }
    }

    /// Best-effort `Drop` server-request for `response_id`: tells the CLI
    /// to tear down the whole response-id bucket (connections + plugin
    /// subprocesses). The reply (`DropResult`) is discarded; transport
    /// errors / timeouts are ignored — teardown is fire-and-forget.
    pub(crate) async fn drop_response(&self, response_id: String) {
        let _ = self
            .request(
                server_request::Payload::Drop(server_request::DropRequest { response_id }),
                IndexMap::new(),
                None,
            )
            .await;
    }

    /// `LaboratoryExportBegin`: start an export on the conduit and get
    /// its transfer id. Each transfer op below is ONE id-correlated
    /// request/response exchange awaited WITHOUT a deadline — laboratory
    /// transfers are timeout-free unconditionally (never bounded by the
    /// per-request MCP call timeout).
    /// `LaboratoryTransfer`: hand the WHOLE cross-host client-to-client
    /// transfer to the CLI daemon, which drives the export/import
    /// splice itself. One request, one `{bytes}` reply — the proxy
    /// never touches payload bytes. Timeout-free like every transfer
    /// op.
    #[allow(clippy::too_many_arguments)]
    pub async fn laboratory_transfer(
        &self,
        request: server_request::LaboratoryTransferRequest,
    ) -> Result<u64, McpError> {
        let response = self
            .request(
                server_request::Payload::LaboratoryTransfer(request),
                IndexMap::new(),
                None,
            )
            .await?;
        match response.payload {
            server_response::Payload::LaboratoryTransfer(result) => {
                unwrap_rpc("laboratory_transfer", result).map(|r| r.bytes)
            }
            other => Err(variant_mismatch(
                "laboratory_transfer",
                "laboratory_transfer",
                &other,
            )),
        }
    }

    /// `LaboratoryLocalTransfer`: both endpoints share one (machine,
    /// state) — the CLI daemon forwards this verbatim to that one
    /// laboratory host, which pipes the bytes container-to-container.
    pub async fn laboratory_local_transfer(
        &self,
        request: server_request::LaboratoryLocalTransferRequest,
    ) -> Result<u64, McpError> {
        let response = self
            .request(
                server_request::Payload::LaboratoryLocalTransfer(request),
                IndexMap::new(),
                None,
            )
            .await?;
        match response.payload {
            server_response::Payload::LaboratoryLocalTransfer(result) => {
                unwrap_rpc("laboratory_local_transfer", result).map(|r| r.bytes)
            }
            other => Err(variant_mismatch(
                "laboratory_local_transfer",
                "laboratory_local_transfer",
                &other,
            )),
        }
    }

    pub async fn laboratory_export_begin(
        &self,
        laboratory_id: String,
        machine: Option<String>,
        machine_state: Option<String>,
        path: String,
    ) -> Result<String, McpError> {
        let response = self
            .request(
                server_request::Payload::LaboratoryExportBegin(
                    server_request::LaboratoryExportBeginRequest {
                        laboratory_id,
                        machine,
                        machine_state,
                        path,
                    },
                ),
                IndexMap::new(),
                None,
            )
            .await?;
        match response.payload {
            server_response::Payload::LaboratoryExportBegin(result) => {
                unwrap_rpc("laboratory_export_begin", result)
                    .map(|r| r.transfer_id)
            }
            other => Err(variant_mismatch(
                "laboratory_export_begin",
                "laboratory_export_begin",
                &other,
            )),
        }
    }

    /// `LaboratoryExportRead`: pull the next chunk.
    pub async fn laboratory_export_read(
        &self,
        transfer_id: String,
    ) -> Result<server_response::LaboratoryExportChunk, McpError> {
        let response = self
            .request(
                server_request::Payload::LaboratoryExportRead(
                    server_request::LaboratoryExportReadRequest { transfer_id },
                ),
                IndexMap::new(),
                None,
            )
            .await?;
        match response.payload {
            server_response::Payload::LaboratoryExportRead(result) => {
                unwrap_rpc("laboratory_export_read", result)
            }
            other => Err(variant_mismatch(
                "laboratory_export_read",
                "laboratory_export_read",
                &other,
            )),
        }
    }

    /// `LaboratoryExportAbort`: best-effort early cleanup.
    pub async fn laboratory_export_abort(&self, transfer_id: String) {
        let _ = self
            .request(
                server_request::Payload::LaboratoryExportAbort(
                    server_request::LaboratoryExportAbortRequest { transfer_id },
                ),
                IndexMap::new(),
                None,
            )
            .await;
    }

    /// `LaboratoryImportBegin`: start an import on the conduit and get
    /// its transfer id.
    pub async fn laboratory_import_begin(
        &self,
        laboratory_id: String,
        machine: Option<String>,
        machine_state: Option<String>,
        path: String,
    ) -> Result<String, McpError> {
        let response = self
            .request(
                server_request::Payload::LaboratoryImportBegin(
                    server_request::LaboratoryImportBeginRequest {
                        laboratory_id,
                        machine,
                        machine_state,
                        path,
                    },
                ),
                IndexMap::new(),
                None,
            )
            .await?;
        match response.payload {
            server_response::Payload::LaboratoryImportBegin(result) => {
                unwrap_rpc("laboratory_import_begin", result)
                    .map(|r| r.transfer_id)
            }
            other => Err(variant_mismatch(
                "laboratory_import_begin",
                "laboratory_import_begin",
                &other,
            )),
        }
    }

    /// `LaboratoryImportWrite`: push one chunk.
    pub async fn laboratory_import_write(
        &self,
        transfer_id: String,
        data: String,
    ) -> Result<(), McpError> {
        let response = self
            .request(
                server_request::Payload::LaboratoryImportWrite(
                    server_request::LaboratoryImportWriteRequest { transfer_id, data },
                ),
                IndexMap::new(),
                None,
            )
            .await?;
        match response.payload {
            server_response::Payload::LaboratoryImportWrite(result) => {
                unwrap_rpc("laboratory_import_write", result).map(|_| ())
            }
            other => Err(variant_mismatch(
                "laboratory_import_write",
                "laboratory_import_write",
                &other,
            )),
        }
    }

    /// `LaboratoryImportEnd`: close the body and get the byte total.
    pub async fn laboratory_import_end(
        &self,
        transfer_id: String,
    ) -> Result<u64, McpError> {
        let response = self
            .request(
                server_request::Payload::LaboratoryImportEnd(
                    server_request::LaboratoryImportEndRequest { transfer_id },
                ),
                IndexMap::new(),
                None,
            )
            .await?;
        match response.payload {
            server_response::Payload::LaboratoryImportEnd(result) => {
                unwrap_rpc("laboratory_import_end", result).map(|r| r.bytes)
            }
            other => Err(variant_mismatch(
                "laboratory_import_end",
                "laboratory_import_end",
                &other,
            )),
        }
    }

    /// `LaboratoryImportAbort`: best-effort early cleanup.
    pub async fn laboratory_import_abort(&self, transfer_id: String) {
        let _ = self
            .request(
                server_request::Payload::LaboratoryImportAbort(
                    server_request::LaboratoryImportAbortRequest { transfer_id },
                ),
                IndexMap::new(),
                None,
            )
            .await;
    }

    /// Hand a proxy-bound `server_response` back to the waiter that
    /// issued the matching request. Called by the API's recv loop.
    /// Unknown id → dropped.
    ///
    /// `Command` frames route to the MULTI-FRAME map: the parked
    /// sender survives every frame until the terminal
    /// [`CommandFrame::Done`] — or until a send fails because the
    /// consumer dropped its stream, which also evicts the entry (and
    /// the daemon's next POST-equivalent frame is dropped here, ending
    /// the exchange silently). Everything else is the classic one
    /// frame per id.
    pub fn deliver_response(&self, response: ServerResponse) {
        let ServerResponse { id, payload } = response;
        if let server_response::Payload::Command { frame } = payload {
            let done = matches!(frame, CommandFrame::Done);
            let dead = match self.0.command_streams.get(&id) {
                Some(tx) => tx.send(frame).is_err(),
                None => false,
            };
            if done || dead {
                self.0.command_streams.remove(&id);
            }
            return;
        }
        if let Some((_, tx)) = self.0.pending.remove(&id) {
            let _ = tx.send(ServerResponse { id, payload });
        }
    }

    /// Execute a CLI command on the CLI daemon on behalf of a
    /// server-side plugin, streaming the response frames back AS THEY
    /// ARRIVE.
    ///
    /// Mints its OWN correlation id — NEVER a plugin-supplied one:
    /// plugin MCP servers are external untrusted code that can't be
    /// trusted to randomize ids, so the proxy↔daemon exchange runs in
    /// the proxy's private id space.
    ///
    /// `ack_timeout` bounds ONLY the wait for the FIRST frame; the
    /// item stream itself carries no artificial deadline.
    ///
    /// First-frame leniency: the daemon returns `Ack` from its
    /// dispatch while a spawned pump emits the remaining frames
    /// through a second writer over the same sink, so a
    /// pathologically fast first item could beat the Ack. ANY first
    /// frame is accepted as evidence the exchange is live: `Ack` →
    /// proceed; `Item` / `Error` / `Done` → proceed with that frame
    /// re-prepended to the stream (a first-frame `Error` from a
    /// RejectHandler-style peer thus surfaces as the run's first
    /// error frame, followed by nothing — the consumer treats the
    /// stream end as done).
    pub(crate) async fn command(
        &self,
        agent_arguments: objectiveai_sdk::cli::command::AgentArguments,
        plugin: objectiveai_sdk::mcp::server::Plugin,
        request: objectiveai_sdk::cli::command::Request,
        ack_timeout: Option<Duration>,
    ) -> Result<impl futures::Stream<Item = CommandFrame> + Send + 'static, McpError>
    {
        let id = uuid::Uuid::new_v4().to_string();
        let (frame_tx, mut frame_rx) = mpsc::unbounded_channel();
        self.0.command_streams.insert(id.clone(), frame_tx);
        let request = ServerRequest {
            id: id.clone(),
            headers: IndexMap::new(),
            payload: server_request::Payload::Command {
                agent_arguments,
                plugin,
                request,
            },
        };
        if self.0.tx.send(request).is_err() {
            self.0.command_streams.remove(&id);
            return Err(transport_error("reverse channel closed before send"));
        }

        let first = match ack_timeout {
            Some(timeout) => {
                match tokio::time::timeout(timeout, frame_rx.recv()).await {
                    Ok(first) => first,
                    Err(_) => {
                        self.0.command_streams.remove(&id);
                        return Err(transport_error(
                            "reverse channel timed out waiting for command ack",
                        ));
                    }
                }
            }
            None => frame_rx.recv().await,
        };
        let Some(first) = first else {
            self.0.command_streams.remove(&id);
            return Err(transport_error(
                "reverse channel dropped before command ack",
            ));
        };
        let prepended = match first {
            CommandFrame::Ack => None,
            other => Some(other),
        };

        // Consumer-drop cleanup is lazy: dropping this stream drops
        // `frame_rx`, and the NEXT delivered frame's failed send
        // evicts the map entry (see `deliver_response`).
        let rest = futures::stream::unfold(frame_rx, |mut rx| async move {
            rx.recv().await.map(|frame| (frame, rx))
        });
        Ok(futures::stream::iter(prepended).chain(rest))
    }

    /// Hand a proxy-bound `client_request` to the proxy.
    ///
    /// `McpListChanged` fires the registered list-changed callback for the
    /// upstream. The MCP-op variants (`ListTools`/`CallTool`/
    /// `ListResources`/`ReadResource`) resolve the session by
    /// `response_id` and run the SAME shared [`crate::session::Session`]
    /// code the HTTP endpoints use — fanning out / routing exactly as
    /// `mcp::handle_tools_list` etc. — returning the normal MCP result.
    /// NOTE: the `CallTool` path deliberately does NOT consult the queue
    /// delegate (that splice is only for the regular HTTP `tools/call`).
    /// Returns the ack/result the API writes back over the WS.
    pub async fn deliver_client_request(
        &self,
        request: client_request::Request,
    ) -> client_response::Response {
        let client_request::Request { id, payload } = request;
        match payload {
            client_request::Payload::McpListChanged(change) => {
                // Exact (response id, kind) first; fall back to the
                // kind-only slot for registrations whose connect
                // headers carried no response id.
                let cbs = self
                    .0
                    .list_changed
                    .get(&(change.response_id.clone(), change.mcp_kind.clone()))
                    .or_else(|| {
                        change.response_id.as_ref()?;
                        self.0.list_changed.get(&(None, change.mcp_kind.clone()))
                    });
                if let Some(cbs) = cbs {
                    let cb = match change.kind {
                        McpListChangedKind::Tools => cbs.0.clone(),
                        McpListChangedKind::Resources => cbs.1.clone(),
                    };
                    drop(cbs);
                    if let Some(cb) = cb {
                        cb();
                    }
                }
                client_response::Response::Ok { id }
            }
            // List params (cursor) are ignored, matching the HTTP
            // `handle_tools_list` which fans out to every upstream.
            // List params (cursor) are ignored, matching the HTTP
            // `handle_tools_list`; `name`, when set, scopes the fan-out to
            // the single server with that routing prefix.
            client_request::Payload::ListTools {
                response_id, name, ..
            } => {
                let result = match self.lookup_session(&response_id).await {
                    Ok(session) => {
                        match session.list_tools_filtered(None, name.as_deref()).await {
                            Ok(result) => JsonRpcResult::Ok { result },
                            Err(e) => rpc_err_result(-32603, format!("list_tools: {e}")),
                        }
                    }
                    Err((code, message)) => rpc_err_result(code, message),
                };
                client_response::Response::ListTools { id, result }
            }
            client_request::Payload::ListResources {
                response_id, name, ..
            } => {
                let result = match self.lookup_session(&response_id).await {
                    Ok(session) => {
                        match session.list_resources_filtered(None, name.as_deref()).await {
                            Ok(result) => JsonRpcResult::Ok { result },
                            Err(e) => rpc_err_result(-32603, format!("list_resources: {e}")),
                        }
                    }
                    Err((code, message)) => rpc_err_result(code, message),
                };
                client_response::Response::ListResources { id, result }
            }
            client_request::Payload::ListServers { response_id } => {
                let result = match self.lookup_session(&response_id).await {
                    // Proxy-local aggregate — no upstream fan-out, can't fail.
                    Ok(session) => JsonRpcResult::Ok {
                        result: session.list_servers(),
                    },
                    Err((code, message)) => rpc_err_result(code, message),
                };
                client_response::Response::ListServers { id, result }
            }
            client_request::Payload::CallTool { response_id, params } => {
                let result = match self.lookup_session(&response_id).await {
                    // No queue delegate here — unlike the HTTP path, this
                    // returns the upstream tool result verbatim.
                    Ok(session) => match session.call_tool(&params).await {
                        Ok(result) => JsonRpcResult::Ok { result },
                        Err(crate::session::CallToolError::ToolNotFound(name)) => {
                            rpc_err_result(-32601, format!("tool not found: {name}"))
                        }
                        Err(crate::session::CallToolError::Upstream(e)) => {
                            rpc_err_result(-32603, format!("upstream call_tool: {e}"))
                        }
                    },
                    Err((code, message)) => rpc_err_result(code, message),
                };
                client_response::Response::CallTool { id, result }
            }
            client_request::Payload::ReadResource { response_id, params } => {
                let result = match self.lookup_session(&response_id).await {
                    Ok(session) => match session.read_resource(&params.uri).await {
                        Ok(result) => JsonRpcResult::Ok { result },
                        Err(crate::session::ReadResourceError::ResourceNotFound(uri)) => {
                            rpc_err_result(-32602, format!("resource not found: {uri}"))
                        }
                        Err(crate::session::ReadResourceError::Upstream(e)) => {
                            rpc_err_result(-32603, format!("upstream read_resource: {e}"))
                        }
                    },
                    Err((code, message)) => rpc_err_result(code, message),
                };
                client_response::Response::ReadResource { id, result }
            }
        }
    }

    fn set_tools_list_changed(
        &self,
        response_id: Option<String>,
        mcp_kind: McpKind,
        cb: ListChangedCb,
    ) {
        let mut entry =
            self.0.list_changed.entry((response_id, mcp_kind)).or_default();
        entry.0 = Some(cb);
    }

    fn set_resources_list_changed(
        &self,
        response_id: Option<String>,
        mcp_kind: McpKind,
        cb: ListChangedCb,
    ) {
        let mut entry =
            self.0.list_changed.entry((response_id, mcp_kind)).or_default();
        entry.1 = Some(cb);
    }
}

/// A `client://`-scheme upstream, reached over the [`ReverseChannel`]. Mirrors
/// the slice of [`Connection`]'s interface the [`crate::session::Session`]
/// uses, translating each op into a `server_request` carrying this
/// upstream's [`McpKind`].
pub struct WsUpstream {
    channel: ReverseChannel,
    mcp_kind: McpKind,
    /// Per-MCP-CALL budget for this upstream's reverse-channel ops
    /// (from the request's `X-MCP-CALL-TIMEOUT` via the proxy config).
    /// `None` ⇒ calls wait forever. Never applied to the connect
    /// (`initialize`) — that uses the connect timeout.
    call_timeout: Option<Duration>,
    /// The `client://…` URL this upstream was dialed with (used for filtering).
    pub url: String,
    /// Upstream `Mcp-Session-Id` returned by the CLI on `initialize`.
    pub session_id: String,
    /// Upstream `server_info.name` / `.version` from the `initialize`
    /// reply — feeds the session's routing-prefix derivation.
    server_name: String,
    server_version: String,
    /// The upstream's full `initialize` reply (capabilities, server_info,
    /// instructions, protocol version) — kept verbatim so `servers/list`
    /// can report it. `Connection` exposes the same via its own
    /// `initialize_result`.
    initialize_result: objectiveai_sdk::mcp::initialize_result::InitializeResult,
    /// Typed laboratory identity from the explicit `X-MCP-Laboratories`
    /// marker — the authoritative "this upstream is a laboratory" signal,
    /// `None` for non-laboratory upstreams. Never derived from the URL.
    laboratory: Option<objectiveai_sdk::laboratories::Laboratory>,
    /// Whether the upstream advertised the `tools` / `resources`
    /// capability in its `initialize` reply. We must NOT issue
    /// `tools/list` / `resources/list` against an upstream that didn't
    /// advertise the capability: many servers (incl. the test
    /// fixtures) 404 the un-advertised method, and a hard error there
    /// fails the whole aggregate — and, on the post-init health probe,
    /// fails the connect and churns endless re-`initialize`s. Mirrors
    /// `mcp::Connection::has_{tools,resources}_cap`.
    has_tools_cap: bool,
    has_resources_cap: bool,
    /// Persistent per-upstream headers captured at connect: the per-URL
    /// set (`Authorization`, custom `X-*`, `X-OBJECTIVEAI-ARGUMENTS`)
    /// plus whatever identity headers were present at dial. Never
    /// mutated after connect — mirrors the SDK `Connection`'s base
    /// `headers`. The transient subset is overridden per request by
    /// `extra_headers`; the per-URL subset has no overlay key, so it
    /// always survives on every request.
    base_headers: IndexMap<String, String>,
    /// Mutable transient-identity overlay, full-replaced every turn by
    /// `apply_transient_headers` → `set_extra_headers`. Overrides
    /// `base_headers` per key (mirrors `Connection::extra_headers`).
    /// Starts empty: until the first refresh, `base_headers` alone
    /// carries the dial-time identity headers.
    extra_headers: RwLock<IndexMap<String, String>>,
}

impl std::fmt::Debug for WsUpstream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WsUpstream")
            .field("url", &self.url)
            .field("session_id", &self.session_id)
            .finish_non_exhaustive()
    }
}

impl WsUpstream {
    /// Headers to stamp on every outbound `server_request`, mirroring
    /// `Connection::build_request_headers` exactly: the persistent
    /// `base_headers` first, then the `extra_headers` transient overlay
    /// (overrides per key), then this upstream's `Mcp-Session-Id` last
    /// (so it can never be shadowed). Per-URL headers live only in
    /// `base_headers` (no overlay key collides), so they're present on
    /// EVERY request — identical to the HTTP path.
    async fn headers(&self) -> IndexMap<String, String> {
        let mut h = self.base_headers.clone();
        for (k, v) in self.extra_headers.read().await.iter() {
            h.insert(k.clone(), v.clone());
        }
        h.insert(
            crate::upstream::MCP_SESSION_ID_KEY.to_string(),
            self.session_id.clone(),
        );
        h
    }

    pub async fn list_tools(&self) -> Result<Arc<Vec<Tool>>, Arc<McpError>> {
        // Capability gate — an upstream that didn't advertise `tools`
        // has no `tools/list`; calling it anyway 404s on most servers.
        if !self.has_tools_cap {
            return Ok(Arc::new(Vec::new()));
        }
        let headers = self.headers().await;
        let response = self
            .channel
            .request(
                server_request::Payload::ToolsList {
                    mcp_kind: self.mcp_kind.clone(),
                    params: ListToolsRequest { cursor: None },
                },
                headers,
                self.call_timeout,
            )
            .await
            .map_err(Arc::new)?;
        match response.payload {
            server_response::Payload::ToolsList { result, .. } => {
                Ok(Arc::new(unwrap_rpc(&self.url, result).map_err(Arc::new)?.tools))
            }
            other => Err(Arc::new(variant_mismatch(&self.url, "tools_list", &other))),
        }
    }

    pub async fn list_resources(&self) -> Result<Arc<Vec<Resource>>, Arc<McpError>> {
        // Capability gate — an upstream that didn't advertise
        // `resources` has no `resources/list`; calling it anyway 404s
        // on most servers (e.g. the tools-only plugin fixtures).
        if !self.has_resources_cap {
            return Ok(Arc::new(Vec::new()));
        }
        let headers = self.headers().await;
        let response = self
            .channel
            .request(
                server_request::Payload::ResourcesList {
                    mcp_kind: self.mcp_kind.clone(),
                    params: ListResourcesRequest { cursor: None },
                },
                headers,
                self.call_timeout,
            )
            .await
            .map_err(Arc::new)?;
        match response.payload {
            server_response::Payload::ResourcesList { result, .. } => {
                Ok(Arc::new(unwrap_rpc(&self.url, result).map_err(Arc::new)?.resources))
            }
            other => Err(Arc::new(variant_mismatch(&self.url, "resources_list", &other))),
        }
    }

    pub async fn call_tool(
        &self,
        params: &CallToolRequestParams,
    ) -> Result<CallToolResult, McpError> {
        let headers = self.headers().await;
        let response = self
            .channel
            .request(
                server_request::Payload::ToolsCall {
                    mcp_kind: self.mcp_kind.clone(),
                    params: params.clone(),
                },
                headers,
                self.call_timeout,
            )
            .await?;
        match response.payload {
            server_response::Payload::ToolsCall { result, .. } => unwrap_rpc(&self.url, result),
            other => Err(variant_mismatch(&self.url, "tools_call", &other)),
        }
    }

    pub async fn read_resource(&self, uri: &str) -> Result<ReadResourceResult, McpError> {
        let headers = self.headers().await;
        let response = self
            .channel
            .request(
                server_request::Payload::ResourcesRead {
                    mcp_kind: self.mcp_kind.clone(),
                    params: ReadResourceRequestParams {
                        uri: uri.to_string(),
                    },
                },
                headers,
                self.call_timeout,
            )
            .await?;
        match response.payload {
            server_response::Payload::ResourcesRead { result, .. } => unwrap_rpc(&self.url, result),
            other => Err(variant_mismatch(&self.url, "resources_read", &other)),
        }
    }

    pub async fn delete(&self) -> Result<(), McpError> {
        let headers = self.headers().await;
        let response = self
            .channel
            .request(
                server_request::Payload::SessionTerminate {
                    mcp_kind: self.mcp_kind.clone(),
                },
                headers,
                self.call_timeout,
            )
            .await?;
        match response.payload {
            server_response::Payload::SessionTerminate { result, .. } => unwrap_rpc(&self.url, result),
            other => Err(variant_mismatch(&self.url, "session_terminate", &other)),
        }
    }

    /// This session's response id, off the connect-time header set —
    /// the registry key half that keeps identical kinds from
    /// colliding across swarm slots on the shared channel.
    fn response_id(&self) -> Option<String> {
        self.base_headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("X-OBJECTIVEAI-RESPONSE-ID"))
            .map(|(_, v)| v.clone())
    }

    pub fn set_on_tools_list_changed<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.channel.set_tools_list_changed(
            self.response_id(),
            self.mcp_kind.clone(),
            Arc::new(callback),
        );
    }

    pub fn set_on_resources_list_changed<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.channel.set_resources_list_changed(
            self.response_id(),
            self.mcp_kind.clone(),
            Arc::new(callback),
        );
    }

    pub async fn set_extra_headers(&self, extras: IndexMap<String, String>) {
        *self.extra_headers.write().await = extras;
    }
}

/// A per-upstream handle: HTTP [`Connection`] or WS [`WsUpstream`]. Exposes
/// exactly the surface [`crate::session::Session`] + `handle_delete` use.
#[derive(Debug)]
pub enum Upstream {
    Http(Connection),
    /// A server-side PLUGIN reached over plain HTTP — marked by the
    /// typed `X-MCP-Plugins` header — connected with a REAL command
    /// executor: when the plugin's MCP server pushes a
    /// `cli_request`, the connection fulfills it by forwarding the
    /// command to the CLI daemon over the reverse channel. (Today all
    /// plugins are client-side `client://` upstreams, so this variant is
    /// future-proofing for server-side plugins.)
    HttpPlugin {
        connection: Connection<crate::command_executor::ReverseChannelCommandExecutor>,
        plugin: objectiveai_sdk::mcp::server::Plugin,
    },
    Ws(WsUpstream),
}

impl Upstream {
    /// Whether this upstream is reached over the `client_objectiveai_mcp`
    /// reverse channel (a `client://` upstream) rather than plain HTTP.
    pub fn is_ws(&self) -> bool {
        matches!(self, Upstream::Ws(_))
    }

    pub fn url(&self) -> &str {
        match self {
            Upstream::Http(c) => &c.url,
            Upstream::HttpPlugin { connection, .. } => &connection.url,
            Upstream::Ws(w) => &w.url,
        }
    }

    pub fn session_id(&self) -> &str {
        match self {
            Upstream::Http(c) => &c.session_id,
            Upstream::HttpPlugin { connection, .. } => &connection.session_id,
            Upstream::Ws(w) => &w.session_id,
        }
    }

    /// Upstream `server_info.name` — used to derive the session's routing
    /// prefix. (`Connection` exposes it via `initialize_result`.)
    pub fn server_name(&self) -> &str {
        match self {
            Upstream::Http(c) => &c.initialize_result.server_info.name,
            Upstream::HttpPlugin { connection, .. } => {
                &connection.initialize_result.server_info.name
            }
            Upstream::Ws(w) => &w.server_name,
        }
    }

    /// Upstream `server_info.version` — the prefix collision tie-breaker.
    pub fn server_version(&self) -> &str {
        match self {
            Upstream::Http(c) => &c.initialize_result.server_info.version,
            Upstream::HttpPlugin { connection, .. } => {
                &connection.initialize_result.server_info.version
            }
            Upstream::Ws(w) => &w.server_version,
        }
    }

    /// The upstream's full `initialize` reply (capabilities, server_info,
    /// instructions, protocol version) — used by `servers/list`.
    pub fn initialize_result(
        &self,
    ) -> &objectiveai_sdk::mcp::initialize_result::InitializeResult {
        match self {
            Upstream::Http(c) => &c.initialize_result,
            Upstream::HttpPlugin { connection, .. } => {
                &connection.initialize_result
            }
            Upstream::Ws(w) => &w.initialize_result,
        }
    }

    /// The laboratory this upstream IS, if any — for `servers/list` and
    /// `laboratory_transfer`.
    ///
    /// Read from the explicit, typed laboratory marker the API supplied
    /// (`X-MCP-Laboratories`), NOT inferred by string-parsing the
    /// `client://laboratory/{id}` URL. HTTP upstreams and plugin
    /// upstreams are `None`.
    pub fn laboratory(&self) -> Option<objectiveai_sdk::laboratories::Laboratory> {
        match self {
            Upstream::Http(_) | Upstream::HttpPlugin { .. } => None,
            Upstream::Ws(w) => w.laboratory.clone(),
        }
    }

    /// The plugin this upstream IS, if any — for `servers/list`. A
    /// websocket upstream whose `McpKind` is `Plugin` (client-side
    /// plugin), or an `HttpPlugin` upstream (server-side plugin,
    /// marked by the typed `X-MCP-Plugins` header).
    pub fn plugin(&self) -> Option<objectiveai_sdk::mcp::server::Plugin> {
        match self {
            Upstream::Http(_) => None,
            Upstream::HttpPlugin { plugin, .. } => Some(plugin.clone()),
            Upstream::Ws(w) => match &w.mcp_kind {
                McpKind::PluginLaboratory {
                    owner,
                    name,
                    version,
                } => Some(objectiveai_sdk::mcp::server::Plugin {
                    owner: owner.clone(),
                    name: name.clone(),
                    version: version.clone(),
                }),
                McpKind::Laboratory { .. } | McpKind::AgentLaboratory { .. } => {
                    None
                }
            },
        }
    }

    /// The session reverse channel this upstream rides, for proxy-level
    /// ops that aren't a per-upstream MCP call (e.g. laboratory transfer,
    /// which spans two laboratories on the same conduit). `None` for HTTP
    /// upstreams, which have no reverse channel. (`HttpPlugin`'s command
    /// EXECUTOR holds a channel, but the upstream itself is not reached
    /// over one — this accessor answers the transfer question, so None.)
    pub fn reverse_channel(&self) -> Option<&ReverseChannel> {
        match self {
            Upstream::Http(_) | Upstream::HttpPlugin { .. } => None,
            Upstream::Ws(w) => Some(&w.channel),
        }
    }

    pub async fn list_tools(&self) -> Result<Arc<Vec<Tool>>, Arc<McpError>> {
        match self {
            Upstream::Http(c) => c.list_tools().await,
            Upstream::HttpPlugin { connection, .. } => connection.list_tools().await,
            Upstream::Ws(w) => w.list_tools().await,
        }
    }

    pub async fn list_resources(&self) -> Result<Arc<Vec<Resource>>, Arc<McpError>> {
        match self {
            Upstream::Http(c) => c.list_resources().await,
            Upstream::HttpPlugin { connection, .. } => connection.list_resources().await,
            Upstream::Ws(w) => w.list_resources().await,
        }
    }

    pub async fn call_tool(
        &self,
        params: &CallToolRequestParams,
    ) -> Result<CallToolResult, McpError> {
        match self {
            Upstream::Http(c) => c.call_tool(params).await,
            Upstream::HttpPlugin { connection, .. } => connection.call_tool(params).await,
            Upstream::Ws(w) => w.call_tool(params).await,
        }
    }

    pub async fn read_resource(&self, uri: &str) -> Result<ReadResourceResult, McpError> {
        match self {
            Upstream::Http(c) => c.read_resource(uri).await,
            Upstream::HttpPlugin { connection, .. } => connection.read_resource(uri).await,
            Upstream::Ws(w) => w.read_resource(uri).await,
        }
    }

    pub async fn delete(&self) -> Result<(), McpError> {
        match self {
            Upstream::Http(c) => c.delete().await,
            Upstream::HttpPlugin { connection, .. } => connection.delete().await,
            Upstream::Ws(w) => w.delete().await,
        }
    }

    pub fn set_on_tools_list_changed<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        match self {
            Upstream::Http(c) => c.set_on_tools_list_changed(callback),
            Upstream::HttpPlugin { connection, .. } => {
                connection.set_on_tools_list_changed(callback)
            }
            Upstream::Ws(w) => w.set_on_tools_list_changed(callback),
        }
    }

    pub fn set_on_resources_list_changed<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        match self {
            Upstream::Http(c) => c.set_on_resources_list_changed(callback),
            Upstream::HttpPlugin { connection, .. } => {
                connection.set_on_resources_list_changed(callback)
            }
            Upstream::Ws(w) => w.set_on_resources_list_changed(callback),
        }
    }

    pub async fn set_extra_headers(&self, extras: IndexMap<String, String>) {
        match self {
            Upstream::Http(c) => c.set_extra_headers(extras).await,
            Upstream::HttpPlugin { connection, .. } => {
                connection.set_extra_headers(extras).await
            }
            Upstream::Ws(w) => w.set_extra_headers(extras).await,
        }
    }
}

/// `initialize` a `client://` upstream over `channel` and build its
/// [`WsUpstream`]. `headers` is the full set sent on the `initialize`
/// request — the session-global transient identity headers, plus (on
/// resume) the upstream `Mcp-Session-Id` and any auth. `args` carries
/// plugin init arguments (empty for laboratories).
///
/// `connect_timeout` bounds the `initialize` round-trip (the per-request
/// call timeout NEVER applies to connects); `call_timeout` is stored on
/// the [`WsUpstream`] for every later op.
pub async fn connect_ws(
    channel: ReverseChannel,
    url: String,
    mcp_kind: McpKind,
    mut headers: IndexMap<String, String>,
    laboratory: Option<objectiveai_sdk::laboratories::Laboratory>,
    connect_timeout: Option<Duration>,
    call_timeout: Option<Duration>,
) -> Result<WsUpstream, McpError> {
    let response = channel
        .request(
            server_request::Payload::Initialize {
                mcp_kind: mcp_kind.clone(),
                params: InitializeRequest::default(),
            },
            headers.clone(),
            connect_timeout,
        )
        .await?;
    let reply = match response.payload {
        server_response::Payload::Initialize { result, .. } => unwrap_rpc(&url, result)?,
        other => return Err(variant_mismatch(&url, "initialize", &other)),
    };
    // The per-request stamped set drops the resume `Mcp-Session-Id`
    // ([`WsUpstream::headers`] re-adds whatever the upstream just minted)
    // but keeps the transient identity + auth so the post-init health
    // probe + every later call still pass the conduit's transient check.
    headers.shift_remove(crate::upstream::MCP_SESSION_ID_KEY);
    let session_id = reply.mcp_session_id;
    let initialize_result = reply.result;
    let has_tools_cap = initialize_result.capabilities.tools.is_some();
    let has_resources_cap = initialize_result.capabilities.resources.is_some();
    let server_name = initialize_result.server_info.name.clone();
    let server_version = initialize_result.server_info.version.clone();
    Ok(WsUpstream {
        channel,
        mcp_kind,
        call_timeout,
        url,
        session_id,
        server_name,
        server_version,
        initialize_result,
        laboratory,
        has_tools_cap,
        has_resources_cap,
        // The connect-time set (per-URL ∪ dial-time identity) is the
        // persistent base; the transient overlay starts empty and is
        // filled by the first `set_extra_headers`. Mirrors the SDK
        // `Connection`, where connect headers are the base and
        // `extra_headers` begins empty.
        base_headers: headers,
        extra_headers: RwLock::new(IndexMap::new()),
    })
}

fn unwrap_rpc<R>(url: &str, result: JsonRpcResult<R>) -> Result<R, McpError> {
    match result {
        JsonRpcResult::Ok { result } => Ok(result),
        JsonRpcResult::Err {
            code,
            message,
            data,
        } => Err(McpError::JsonRpc {
            url: url.to_string(),
            code,
            message,
            data,
        }),
    }
}

pub(crate) fn transport_error(message: &str) -> McpError {
    McpError::MalformedResponse {
        url: "ws".to_string(),
        message: message.to_string(),
    }
}

/// Build a `JsonRpcResult::Err` for an inbound MCP-op `client_request`
/// (`deliver_client_request`). Generic over the result type so each
/// op's reply variant infers `R`.
fn rpc_err_result<R>(code: i64, message: String) -> JsonRpcResult<R> {
    JsonRpcResult::Err {
        code,
        message,
        data: None,
    }
}

fn variant_mismatch(url: &str, expected: &str, got: &server_response::Payload) -> McpError {
    McpError::MalformedResponse {
        url: url.to_string(),
        message: format!(
            "reverse channel returned wrong payload variant: expected {expected}, got {}",
            got_variant_name(got),
        ),
    }
}

fn got_variant_name(p: &server_response::Payload) -> &'static str {
    use server_response::Payload as P;
    match p {
        P::Initialize { .. } => "initialize",
        P::ToolsList { .. } => "tools_list",
        P::ToolsCall { .. } => "tools_call",
        P::ResourcesList { .. } => "resources_list",
        P::ResourcesRead { .. } => "resources_read",
        P::SessionTerminate { .. } => "session_terminate",
        P::ReadMessageQueue(_) => "read_message_queue",
        P::Retrieve(_) => "retrieve",
        P::Script(_) => "script",
        P::Drop(_) => "drop",
        P::LaboratoryTransfer(_) => "laboratory_transfer",
        P::LaboratoryLocalTransfer(_) => "laboratory_local_transfer",
        P::LaboratoryExportBegin(_) => "laboratory_export_begin",
        P::LaboratoryExportRead(_) => "laboratory_export_read",
        P::LaboratoryExportAbort(_) => "laboratory_export_abort",
        P::LaboratoryImportBegin(_) => "laboratory_import_begin",
        P::LaboratoryImportWrite(_) => "laboratory_import_write",
        P::LaboratoryImportEnd(_) => "laboratory_import_end",
        P::LaboratoryImportAbort(_) => "laboratory_import_abort",
        P::Command { .. } => "command",
    }
}
