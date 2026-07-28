//! EPHEMERAL laboratories: one laboratory per agent-completion
//! response id, for agent-embedded and plugin laboratories.
//!
//! The lifetime model is the whole point, and it is deliberately NOT
//! the [`LabServer`](crate::server::LabServer) one:
//!
//! - Created and MCP-CONNECTED in one atomic host-level op
//!   (`AgentEphemeralCreate` / `PluginEphemeralCreate`) — the op
//!   passes only when BOTH exist, and its reply carries the identity
//!   AND the initialize result.
//! - EXACTLY ONE MCP connection, ever, established at create. A later
//!   `Initialize` addressed at the laboratory is an error.
//! - The laboratory's lifetime IS that connection's: the moment it
//!   ends (SessionTerminate, its Drop, the owning daemon channel
//!   dying, host shutdown) the container EVAPORATES — `podman rm -f`,
//!   zero grace (vs the regular 30s stop-and-keep).
//! - No lazy start, no idle stop, no OnceCell — none of the regular
//!   lifecycle machinery applies.
//!
//! Side surfaces are kept while the laboratory lives, id-agnostic:
//! transfers work against any live laboratory (and die with it), and
//! agent ephemerals run the filetree pump (they carry the injected
//! `objectiveai-mcp-laboratory`; plugin ephemerals run their own
//! image's entrypoint and have no such surface). NEITHER extends the
//! lifetime — the MCP connection is the sole driver.

use std::sync::Arc;

use indexmap::IndexMap;
use objectiveai_sdk::laboratories::daemon::{
    ChannelRequest, ChannelResponse, DropResult, JsonRpcResult, RequestPayload,
    ResponsePayload, TransferAck,
};
use objectiveai_sdk::mcp::resource::{
    ListResourcesRequest, ListResourcesResult, ReadResourceRequestParams, ReadResourceResult,
};
use objectiveai_sdk::mcp::tool::{
    CallToolRequestParams, CallToolResult, ListToolsRequest, ListToolsResult,
};

use crate::upstream::{raw_call, response_id_from_headers, rpc_err};

/// One live ephemeral laboratory (its id — which embeds the response
/// id — is the `HostServer::ephemerals` map key). Registered by the
/// create op; removed (and its container `rm -f`ed) by
/// `HostServer::evaporate`.
pub struct EphemeralLab {
    /// The agent-completion response id this laboratory serves — the
    /// only response id whose ops it accepts.
    pub response_id: String,
    /// The daemon channel that created it — that channel's death
    /// evaporates it.
    pub channel: u64,
    /// The container's loopback HTTP base.
    pub base_url: String,
    /// THE connection — the one and only, made at create.
    connection:
        Arc<objectiveai_sdk::mcp::Connection<crate::host_command::HostCommandExecutor>>,
    /// Plugin ephemerals only: the live header bag the command
    /// executor reads at execute time, full-replaced on every op.
    transient: Option<Arc<tokio::sync::RwLock<IndexMap<String, String>>>>,
    /// Parked transfer halves — alive exactly as long as the lab.
    pub transfers: crate::transfer::Transfers,
    /// Plugin ephemerals only: the per-plugin Postgres tunnel proxy.
    /// `None` for agent ephemerals. Dropping it (or sending its
    /// cancel) tears down the listener + every live stream.
    pub pg: Option<PgProxy>,
}

/// A plugin ephemeral's Postgres tunnel proxy: the host TCP port the
/// container dials (`OBJECTIVEAI_POSTGRES_URL`) plus the ONE cancel
/// governing its accept task and every per-connection pump.
pub struct PgProxy {
    pub port: u16,
    pub cancel: tokio::sync::watch::Sender<bool>,
}

impl EphemeralLab {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        response_id: String,
        channel: u64,
        base_url: String,
        connection: objectiveai_sdk::mcp::Connection<
            crate::host_command::HostCommandExecutor,
        >,
        transient: Option<Arc<tokio::sync::RwLock<IndexMap<String, String>>>>,
        pg: Option<PgProxy>,
    ) -> Self {
        Self {
            transfers: crate::transfer::Transfers::new(base_url.clone()),
            response_id,
            channel,
            base_url,
            connection: Arc::new(connection),
            transient,
            pg,
        }
    }

    /// Serve one lab-scoped request. The host has already demuxed on
    /// `laboratory_id` AND intercepted the lifetime-ending ops
    /// (`SessionTerminate` / `Drop` evaporate at the HostServer, which
    /// owns the registry) — reaching those arms here is a routing bug.
    ///
    /// Every MCP op verifies the request's response-id header against
    /// the laboratory's own: an ephemeral serves exactly ONE
    /// completion, and someone else's response id has no business on
    /// this connection.
    pub async fn handle(&self, request: ChannelRequest) -> ChannelResponse {
        let ChannelRequest { id, headers, payload, .. } = request;
        let payload = match payload {
            RequestPayload::Initialize => ResponsePayload::Initialize(rpc_err(
                -32001,
                "ephemeral laboratories accept exactly one connection, established at create"
                    .into(),
            )),
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
            // Transfer ops are id-agnostic by decision — no response-id
            // check; they die with the laboratory.
            RequestPayload::ExportBegin(req) => self.transfers.export_begin(req).await,
            RequestPayload::ExportRead(req) => self.transfers.export_read(req).await,
            RequestPayload::ExportAbort(req) => self.transfers.export_abort(req),
            RequestPayload::ImportBegin(req) => self.transfers.import_begin(req).await,
            RequestPayload::ImportWrite(req) => self.transfers.import_write(req).await,
            RequestPayload::ImportEnd(req) => self.transfers.import_end(req).await,
            RequestPayload::ImportAbort(req) => self.transfers.import_abort(req),
            // Intercepted by the HostServer (they END the laboratory);
            // reaching here is a routing bug, but the reply shape
            // still pairs correctly.
            RequestPayload::SessionTerminate => ResponsePayload::SessionTerminate(
                rpc_err(-32601, "ephemeral terminate is served by the host".into()),
            ),
            RequestPayload::Drop(_) => ResponsePayload::Drop(DropResult {
                dropped: false,
            }),
            // The host short-circuits filetree watch state for live
            // ephemerals (an ack — watches never drive their
            // lifetime); reaching here is a routing bug.
            RequestPayload::Filetree(_) => ResponsePayload::Filetree(JsonRpcResult::Ok {
                result: TransferAck {},
            }),
            // Host-level ops — answered by the HostServer BEFORE the
            // per-lab demux; reaching here is a routing bug, but the
            // reply shape still pairs correctly.
            RequestPayload::Create(_) => ResponsePayload::Create(rpc_err(
                -32601,
                "ephemeral laboratory does not serve create (host-level op)".into(),
            )),
            RequestPayload::AgentEphemeralCreate(_) => {
                ResponsePayload::AgentEphemeralCreate(rpc_err(
                    -32601,
                    "ephemeral laboratory does not serve ephemeral create (host-level op)"
                        .into(),
                ))
            }
            RequestPayload::PluginEphemeralCreate(_) => {
                ResponsePayload::PluginEphemeralCreate(rpc_err(
                    -32601,
                    "ephemeral laboratory does not serve ephemeral create (host-level op)"
                        .into(),
                ))
            }
            RequestPayload::PluginImageReset(_) => {
                ResponsePayload::PluginImageReset(rpc_err(
                    -32601,
                    "ephemeral laboratory does not serve plugin image reset \
                     (host-level op)"
                        .into(),
                ))
            }
            RequestPayload::Delete(_) => ResponsePayload::Delete(rpc_err(
                -32601,
                "ephemeral laboratory does not serve delete (host-level op)".into(),
            )),
            RequestPayload::LocalTransfer(_) => ResponsePayload::LocalTransfer(rpc_err(
                -32601,
                "ephemeral laboratory does not serve local transfer (host-level op)".into(),
            )),
            RequestPayload::BuildCreate(_) => ResponsePayload::BuildCreate(rpc_err(
                -32601,
                "ephemeral laboratory does not serve viewer-plugin builds (host-level op)".into(),
            )),
            RequestPayload::BuildRead(_) => ResponsePayload::BuildRead(rpc_err(
                -32601,
                "ephemeral laboratory does not serve viewer-plugin builds (host-level op)".into(),
            )),
            RequestPayload::BuildAbort(_) => ResponsePayload::BuildAbort(rpc_err(
                -32601,
                "ephemeral laboratory does not serve viewer-plugin builds (host-level op)".into(),
            )),
        };
        ChannelResponse { id, payload }
    }

    /// Raw JSON-RPC POST through THE connection, gated on the
    /// response id.
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
        if response_id_from_headers(headers).as_deref() != Some(self.response_id.as_str())
        {
            return rpc_err(
                -32001,
                "response id does not own this ephemeral laboratory".into(),
            );
        }
        // Plugin ephemerals: full-replace the live header bag BEFORE
        // the upstream call — a cli_request the plugin fires while
        // serving this op reads the op's own agent identity.
        if let Some(transient) = &self.transient {
            *transient.write().await = headers.clone();
        }
        match raw_call(&self.connection, headers, method, params).await {
            Ok(result) => result,
            Err(message) => rpc_err(-32603, format!("laboratory: {message}")),
        }
    }
}
