//! The per-laboratory request server for REGULAR laboratories — a
//! mini-conduit for ONE laboratory.
//!
//! Every laboratory-scoped [`ChannelRequest`] arriving over a daemon
//! `/laboratory` WS lands here, after the
//! [`HostServer`](crate::host::HostServer) demuxes it by
//! `laboratory_id` (EPHEMERAL laboratories demux to
//! [`EphemeralLab`](crate::ephemeral::EphemeralLab) instead). MCP ops
//! run against the container's MCP server on its published loopback
//! port through per-response-id
//! [`objectiveai_sdk::mcp::Connection`]s; the transfer ops ride the
//! shared [`crate::transfer::Transfers`] registry.

use std::sync::Arc;

use dashmap::DashMap;
use indexmap::IndexMap;
use objectiveai_sdk::laboratories::daemon::{
    ChannelRequest, ChannelResponse, DropResult, InitializeReply, JsonRpcResult,
    RequestPayload, ResponsePayload,
};
use objectiveai_sdk::mcp::resource::{
    ListResourcesRequest, ListResourcesResult, ReadResourceRequestParams, ReadResourceResult,
};
use objectiveai_sdk::mcp::tool::{
    CallToolRequestParams, CallToolResult, ListToolsRequest, ListToolsResult,
};

use crate::upstream::{
    raw_call, response_id_from_headers, rpc_err, sanitize_connect_headers,
};

/// What makes a [`LabServer`] a PLUGIN laboratory's: the shared
/// command bridge plus the plugin's coordinates. Its sessions connect
/// with the command-forwarding executor
/// ([`crate::host_command::HostCommandExecutor`]) so the plugin's MCP
/// server can run CLI commands on the daemon; regular laboratories
/// (`None`) connect with the executor's inert form.
pub struct PluginSeed {
    pub bridge: Arc<crate::host_command::CommandBridge>,
    pub plugin: objectiveai_sdk::mcp::server::Plugin,
}

/// One per-response-id MCP session into the container.
struct SessionEntry {
    /// The daemon channel that opened it — a channel disconnect drops
    /// its sessions ([`LabServer::drop_channel`]), so a dead daemon
    /// can never pin this container running.
    channel: u64,
    connection:
        Arc<objectiveai_sdk::mcp::Connection<crate::host_command::HostCommandExecutor>>,
    /// Plugin sessions only: the session's LATEST request headers,
    /// full-replaced on every op and read by the executor at
    /// `execute()` time — the freshest agent identity always wins
    /// (the proxy's transient-bag semantics). `None` on regular labs.
    transient:
        Option<Arc<tokio::sync::RwLock<IndexMap<String, String>>>>,
}

/// The one-laboratory server: MCP session registry + transfer registry
/// + the container's loopback base URL.
pub struct LabServer {
    /// The host-authoritative laboratory id — stamped on outbound
    /// [`objectiveai_sdk::laboratories::daemon::HostNotification::McpListChanged`]
    /// frames so the daemon's mirror can resolve the wire kind.
    id: String,
    /// The container's MCP/transfer HTTP base (`http://127.0.0.1:{port}`).
    base_url: String,
    mcp: objectiveai_sdk::mcp::Client,
    /// The host-wide command bridge — reached for its control-lane
    /// senders by the per-session list-changed forwarders (a SENDER is
    /// captured, never this struct or the host; see
    /// [`crate::upstream::install_list_changed_forwarders`]).
    bridge: Arc<crate::host_command::CommandBridge>,
    /// `Some` ⇒ this laboratory IS a plugin (see [`PluginSeed`]).
    plugin: Option<PluginSeed>,
    /// Per-response-id MCP connections into the container.
    connections: DashMap<String, SessionEntry>,
    /// Parked transfer halves, keyed by manager-minted transfer id.
    transfers: crate::transfer::Transfers,
}

impl LabServer {
    pub fn new(
        id: String,
        base_url: String,
        bridge: Arc<crate::host_command::CommandBridge>,
        plugin: Option<PluginSeed>,
    ) -> Self {
        Self {
            mcp: crate::upstream::lab_mcp_client(),
            transfers: crate::transfer::Transfers::new(base_url.clone()),
            id,
            base_url,
            bridge,
            plugin,
            connections: DashMap::new(),
        }
    }

    /// The container's loopback HTTP base — the host's local-transfer
    /// path pipes between labs by base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Whether any per-response MCP connection is live — half of the
    /// host's container-idle condition (the other half is the
    /// filetree-watch state, which lives on the HostServer).
    pub fn has_connections(&self) -> bool {
        !self.connections.is_empty()
    }

    /// Drop every session `channel` opened — called when that daemon
    /// channel disconnects (its response ids are unreachable until the
    /// daemon reconnects and re-initializes).
    pub fn drop_channel(&self, channel: u64) {
        self.connections.retain(|_, entry| entry.channel != channel);
    }

    /// Whether any transfer half is parked — in-flight exports/imports
    /// are demand too: they hold no MCP connection, and an idle stop
    /// mid-stream would truncate them.
    pub fn has_transfers(&self) -> bool {
        !self.transfers.is_empty()
    }

    /// Serve one request; the reply echoes the correlation id. The
    /// host has already demuxed on `laboratory_id` — this server IS
    /// that laboratory's. `channel` tags any session the request opens
    /// with its daemon channel (see [`Self::drop_channel`]).
    pub async fn handle(self: &Arc<Self>, channel: u64, request: ChannelRequest) -> ChannelResponse {
        let ChannelRequest { id, headers, payload, .. } = request;
        let payload = match payload {
            RequestPayload::Initialize => self.initialize(channel, &headers).await,
            RequestPayload::SessionTerminate => self.session_terminate(&headers).await,
            RequestPayload::ToolsList(params) => ResponsePayload::ToolsList(
                self.call::<ListToolsRequest, ListToolsResult>(&headers, "tools/list", &params)
                    .await,
            ),
            RequestPayload::ToolsCall(params) => ResponsePayload::ToolsCall(
                self.call::<CallToolRequestParams, CallToolResult>(
                    &headers,
                    "tools/call",
                    &params,
                )
                .await,
            ),
            RequestPayload::ResourcesList(params) => ResponsePayload::ResourcesList(
                self.call::<ListResourcesRequest, ListResourcesResult>(
                    &headers,
                    "resources/list",
                    &params,
                )
                .await,
            ),
            RequestPayload::ResourcesRead(params) => ResponsePayload::ResourcesRead(
                self.call::<ReadResourceRequestParams, ReadResourceResult>(
                    &headers,
                    "resources/read",
                    &params,
                )
                .await,
            ),
            RequestPayload::Drop(req) => {
                // Drop = kill for this response id's session, no upstream
                // DELETE (mirrors the conduit's Drop semantics).
                let dropped = self.connections.remove(&req.response_id).is_some();
                ResponsePayload::Drop(DropResult { dropped })
            }
            RequestPayload::ExportBegin(req) => self.transfers.export_begin(req).await,
            RequestPayload::ExportRead(req) => self.transfers.export_read(req).await,
            RequestPayload::ExportAbort(req) => self.transfers.export_abort(req),
            RequestPayload::ImportBegin(req) => self.transfers.import_begin(req).await,
            RequestPayload::ImportWrite(req) => self.transfers.import_write(req).await,
            RequestPayload::ImportEnd(req) => self.transfers.import_end(req).await,
            RequestPayload::ImportAbort(req) => self.transfers.import_abort(req),
            // Answered by the HostServer BEFORE the per-lab demux
            // (it owns the filetree-watch state); reaching here is a
            // routing bug, but the reply shape still pairs correctly.
            RequestPayload::Filetree(_) => ResponsePayload::Filetree(rpc_err(
                -32601,
                "laboratory server does not serve filetree watch state (host-level op)".into(),
            )),
            // Host-level ops — answered by the HostServer BEFORE the
            // per-lab demux; reaching here is a routing bug, but the
            // reply shape still pairs correctly.
            RequestPayload::Create(_) => ResponsePayload::Create(rpc_err(
                -32601,
                "laboratory server does not serve create (host-level op)".into(),
            )),
            RequestPayload::AgentEphemeralCreate(_) => {
                ResponsePayload::AgentEphemeralCreate(rpc_err(
                    -32601,
                    "laboratory server does not serve ephemeral create (host-level op)"
                        .into(),
                ))
            }
            RequestPayload::PluginEphemeralCreate(_) => {
                ResponsePayload::PluginEphemeralCreate(rpc_err(
                    -32601,
                    "laboratory server does not serve ephemeral create (host-level op)"
                        .into(),
                ))
            }
            RequestPayload::PluginImageReset(_) => {
                ResponsePayload::PluginImageReset(rpc_err(
                    -32601,
                    "laboratory server does not serve plugin image reset (host-level op)"
                        .into(),
                ))
            }
            RequestPayload::Delete(_) => ResponsePayload::Delete(rpc_err(
                -32601,
                "laboratory server does not serve delete (host-level op)".into(),
            )),
            RequestPayload::LocalTransfer(_) => ResponsePayload::LocalTransfer(rpc_err(
                -32601,
                "laboratory server does not serve local transfer (host-level op)".into(),
            )),
            RequestPayload::BuildCreate(_) => ResponsePayload::BuildCreate(rpc_err(
                -32601,
                "laboratory server does not serve viewer-plugin builds (host-level op)".into(),
            )),
            RequestPayload::BuildRead(_) => ResponsePayload::BuildRead(rpc_err(
                -32601,
                "laboratory server does not serve viewer-plugin builds (host-level op)".into(),
            )),
            RequestPayload::BuildAbort(_) => ResponsePayload::BuildAbort(rpc_err(
                -32601,
                "laboratory server does not serve viewer-plugin builds (host-level op)".into(),
            )),
        };
        ChannelResponse { id, payload }
    }

    // ── MCP session ops ──────────────────────────────────────────

    async fn initialize(
        &self,
        channel: u64,
        headers: &IndexMap<String, String>,
    ) -> ResponsePayload {
        let initialize_err = |code: i64, message: String| {
            ResponsePayload::Initialize(JsonRpcResult::Err { code, message, data: None })
        };
        let Some(response_id) = response_id_from_headers(headers) else {
            return initialize_err(-32600, "missing X-OBJECTIVEAI-RESPONSE-ID header".into());
        };
        let connect_headers = sanitize_connect_headers(headers);
        // The session's executor: plugin labs get the real
        // command-forwarder (owned by THIS daemon channel, reading the
        // session's live header bag at execute time); regular labs get
        // the inert form — their container MCP never requests commands.
        let (executor, transient) = match &self.plugin {
            Some(seed) => {
                let transient = Arc::new(tokio::sync::RwLock::new(headers.clone()));
                (
                    crate::host_command::HostCommandExecutor {
                        inner: Some(Arc::new(
                            crate::host_command::PluginExecutorState {
                                bridge: Arc::clone(&seed.bridge),
                                plugin: seed.plugin.clone(),
                                channel,
                                transient: Arc::clone(&transient),
                            },
                        )),
                    },
                    Some(transient),
                )
            }
            None => (
                crate::host_command::HostCommandExecutor { inner: None },
                None,
            ),
        };
        let connection = match self
            .mcp
            .clone()
            .with_executor(executor)
            .connect(format!("{}/", self.base_url), None, Some(connect_headers))
            .await
        {
            Ok(c) => c,
            Err(e) => return initialize_err(-32603, format!("connect: {e}")),
        };
        // First hop of the list-changed relay — installed on every
        // session (regular AND plugin labs), before the session is
        // visible in the registry.
        crate::upstream::install_list_changed_forwarders(
            &self.bridge,
            channel,
            &self.id,
            &response_id,
            &connection,
        );
        let mcp_session_id = connection.session_id.clone();
        let result = connection.initialize_result.clone();
        self.connections.insert(
            response_id.clone(),
            SessionEntry {
                channel,
                connection: Arc::new(connection),
                transient,
            },
        );
        // Detach-race net (`finish_ephemeral`'s twin): the owning
        // channel may have died during the connect await, in which
        // case its `drop_channel` retain already ran and would never
        // see this insert — the session would pin `has_connections()`
        // true forever, defeating the idle stop. `detach_channel`
        // removes the outbound sender FIRST, then sweeps: if we still
        // see the sender here, any later sweep runs after our insert
        // and retains us away; if it is gone, we self-clean.
        if !self.bridge.outbound.contains_key(&channel) {
            self.connections.remove(&response_id);
            return initialize_err(
                -32603,
                "owning daemon channel disconnected".to_string(),
            );
        }
        ResponsePayload::Initialize(JsonRpcResult::Ok {
            result: InitializeReply {
                mcp_session_id,
                result,
            },
        })
    }

    async fn session_terminate(
        &self,
        headers: &IndexMap<String, String>,
    ) -> ResponsePayload {
        let ok = || ResponsePayload::SessionTerminate(JsonRpcResult::Ok { result: () });
        let Some(response_id) = response_id_from_headers(headers) else {
            return ok();
        };
        let Some((connection, transient)) = self
            .connections
            .get(&response_id)
            .map(|entry| (Arc::clone(&entry.connection), entry.transient.clone()))
        else {
            return ok();
        };
        if let Some(transient) = transient {
            *transient.write().await = headers.clone();
        }
        match connection.delete().await {
            Ok(()) => {
                self.connections.remove(&response_id);
                ok()
            }
            Err(e) => ResponsePayload::SessionTerminate(JsonRpcResult::Err {
                code: -32603,
                message: format!("laboratory: upstream delete: {e}"),
                data: None,
            }),
        }
    }

    /// Raw JSON-RPC POST through the response id's connection —
    /// the conduit's `upstream_call`, scoped to this laboratory.
    async fn call<P, R>(
        &self,
        headers: &IndexMap<String, String>,
        method: &str,
        params: &P,
    ) -> JsonRpcResult<R>
    where
        P: serde::Serialize,
        R: serde::de::DeserializeOwned,
    {
        let Some(response_id) = response_id_from_headers(headers) else {
            return rpc_err(-32600, "missing X-OBJECTIVEAI-RESPONSE-ID header".into());
        };
        let Some((conn, transient)) = self
            .connections
            .get(&response_id)
            .map(|entry| (Arc::clone(&entry.connection), entry.transient.clone()))
        else {
            return rpc_err(
                -32001,
                format!("no cached connection for response id {response_id:?}"),
            );
        };
        // Plugin sessions: full-replace the live header bag BEFORE the
        // upstream call — a cli_request the plugin fires while serving
        // this op reads the op's own agent identity.
        if let Some(transient) = transient {
            *transient.write().await = headers.clone();
        }
        match raw_call(&conn, headers, method, params).await {
            Ok(result) => result,
            Err(message) => rpc_err(-32603, format!("laboratory: {message}")),
        }
    }
}
