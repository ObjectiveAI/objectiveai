//! The `/laboratory` channel's OWN request/response vocabulary.
//!
//! The daemon↔host channel and the API↔client reverse channel are
//! NAIVE to each other: neither imports the other's payloads, and the
//! CLI daemon's conduit is the one place an API-side op is translated
//! into a host-side op (and the response back). The only types both
//! channels share are the provider-neutral MCP domain models
//! (`crate::mcp::tool` / `resource` / `initialize_result`) — genuine
//! data, not channel vocabulary. Everything else here, down to the
//! [`JsonRpcResult`] envelope, is a deliberate local definition.
//!
//! What the vocabulary deliberately does NOT carry (the reverse
//! channel needs these, the host does not):
//! - `mcp_kind` — reverse-channel routing metadata; the conduit
//!   re-attaches it when translating a response back.
//! - `machine` / `machine_state` — daemon-side routing; by the time a
//!   frame reaches a host, the host IS the (machine, state).
//! - per-payload `laboratory_id` — the [`ChannelRequest`](super::ChannelRequest)
//!   envelope's `laboratory_id` addresses the lab.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Daemon → host: every op the `/laboratory` channel can carry.
/// Container-scoped ops (everything except `Create` / `Delete` /
/// `LocalTransfer`) demux by the envelope's `laboratory_id`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.RequestPayload")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RequestPayload {
    /// Open (or reuse) the per-response MCP connection into the
    /// laboratory container. The host dials fresh — no params.
    #[schemars(title = "Initialize")]
    Initialize,
    /// Tear down the per-response MCP connection.
    #[schemars(title = "SessionTerminate")]
    SessionTerminate,
    #[schemars(title = "ToolsList")]
    ToolsList(crate::mcp::tool::ListToolsRequest),
    #[schemars(title = "ToolsCall")]
    ToolsCall(crate::mcp::tool::CallToolRequestParams),
    #[schemars(title = "ResourcesList")]
    ResourcesList(crate::mcp::resource::ListResourcesRequest),
    #[schemars(title = "ResourcesRead")]
    ResourcesRead(crate::mcp::resource::ReadResourceRequestParams),
    /// Drop every per-response connection for `response_id` on this
    /// laboratory — the host-side half of the conduit's own drop.
    #[schemars(title = "Drop")]
    Drop(DropRequest),
    /// Begin streaming a tar export OUT of the laboratory: the host
    /// opens the container's `/export` and parks the byte stream under
    /// a fresh `transfer_id`; the requester then PULLS chunks with
    /// [`RequestPayload::ExportRead`].
    #[schemars(title = "ExportBegin")]
    ExportBegin(ExportBeginRequest),
    /// Pull the next chunk of a parked export. `eof: true` on the
    /// reply means the stream completed and the entry is gone (the
    /// final reply's `data` may still be non-empty).
    #[schemars(title = "ExportRead")]
    ExportRead(TransferIdRequest),
    /// Drop a parked export early (requester-side failure).
    #[schemars(title = "ExportAbort")]
    ExportAbort(TransferIdRequest),
    /// Begin streaming a tar import INTO the laboratory: the host
    /// opens the container's `/import` with a channel-backed body and
    /// parks the sender under a fresh `transfer_id`.
    #[schemars(title = "ImportBegin")]
    ImportBegin(ImportBeginRequest),
    /// Push one chunk into a parked import body.
    #[schemars(title = "ImportWrite")]
    ImportWrite(ImportWriteRequest),
    /// Close a parked import and await the unpack; replies with the
    /// total bytes fed.
    #[schemars(title = "ImportEnd")]
    ImportEnd(TransferIdRequest),
    /// Drop a parked import early — the truncated tar fails the
    /// unpack, so nothing partial is kept silently.
    #[schemars(title = "ImportAbort")]
    ImportAbort(TransferIdRequest),
    /// Create a laboratory on this host (podman create + MCP binary
    /// injection, container NOT started). Host-level: no envelope
    /// `laboratory_id`.
    #[schemars(title = "Create")]
    Create(CreateRequest),
    /// Delete a laboratory from this host (podman rm). Host-level.
    #[schemars(title = "Delete")]
    Delete(DeleteRequest),
    /// Transfer a path between TWO laboratories that both live on
    /// THIS host: the host pipes the source's export stream straight
    /// into the destination's import — no chunk staging anywhere.
    /// Host-level: it addresses two labs, never the envelope's one.
    #[schemars(title = "LocalTransfer")]
    LocalTransfer(LocalTransferRequest),
}

/// Host → daemon: the reply vocabulary, 1:1 with [`RequestPayload`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.ResponsePayload")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponsePayload {
    #[schemars(title = "Initialize")]
    Initialize(JsonRpcResult<InitializeReply>),
    #[schemars(title = "SessionTerminate")]
    SessionTerminate(JsonRpcResult<()>),
    #[schemars(title = "ToolsList")]
    ToolsList(JsonRpcResult<crate::mcp::tool::ListToolsResult>),
    #[schemars(title = "ToolsCall")]
    ToolsCall(JsonRpcResult<crate::mcp::tool::CallToolResult>),
    #[schemars(title = "ResourcesList")]
    ResourcesList(JsonRpcResult<crate::mcp::resource::ListResourcesResult>),
    #[schemars(title = "ResourcesRead")]
    ResourcesRead(JsonRpcResult<crate::mcp::resource::ReadResourceResult>),
    /// Infallible — reports whether a bucket was present and removed.
    #[schemars(title = "Drop")]
    Drop(DropResult),
    #[schemars(title = "ExportBegin")]
    ExportBegin(JsonRpcResult<TransferBeginResult>),
    #[schemars(title = "ExportRead")]
    ExportRead(JsonRpcResult<ExportChunk>),
    #[schemars(title = "ExportAbort")]
    ExportAbort(JsonRpcResult<TransferAck>),
    #[schemars(title = "ImportBegin")]
    ImportBegin(JsonRpcResult<TransferBeginResult>),
    #[schemars(title = "ImportWrite")]
    ImportWrite(JsonRpcResult<TransferAck>),
    #[schemars(title = "ImportEnd")]
    ImportEnd(JsonRpcResult<ImportEndResult>),
    #[schemars(title = "ImportAbort")]
    ImportAbort(JsonRpcResult<TransferAck>),
    #[schemars(title = "Create")]
    Create(JsonRpcResult<super::Identify>),
    #[schemars(title = "Delete")]
    Delete(JsonRpcResult<TransferAck>),
    #[schemars(title = "LocalTransfer")]
    LocalTransfer(JsonRpcResult<LocalTransferResult>),
}

/// JSON-RPC result/error shape for this channel's typed methods —
/// the host channel's OWN envelope, deliberately not shared with the
/// reverse channel's twin.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.JsonRpcResult.{R}", bound = "R: JsonSchema")]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum JsonRpcResult<R> {
    /// Method returned a typed result.
    #[schemars(title = "Ok")]
    Ok { result: R },
    /// Method returned a JSON-RPC error envelope.
    #[schemars(title = "Err")]
    Err {
        code: i64,
        message: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        data: Option<serde_json::Value>,
    },
}

/// Successful payload for [`ResponsePayload::Initialize`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.InitializeReply")]
pub struct InitializeReply {
    /// The container upstream's native `Mcp-Session-Id`.
    pub mcp_session_id: String,
    /// The container's verbatim `InitializeResult`.
    pub result: crate::mcp::initialize_result::InitializeResult,
}

/// Parameters for [`RequestPayload::Drop`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.DropRequest")]
pub struct DropRequest {
    pub response_id: String,
}

/// Payload for [`ResponsePayload::Drop`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.DropResult")]
pub struct DropResult {
    pub dropped: bool,
}

/// Parameters for [`RequestPayload::ExportBegin`]. `path` exports
/// cp-style (a directory is archived under its basename).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.ExportBeginRequest")]
pub struct ExportBeginRequest {
    pub path: String,
}

/// Parameters for [`RequestPayload::ImportBegin`]. The tar unpacks
/// into `path` (created if missing).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.ImportBeginRequest")]
pub struct ImportBeginRequest {
    pub path: String,
}

/// Parameters for every transfer op addressed purely by its parked
/// `transfer_id` (read / end / both aborts).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.TransferIdRequest")]
pub struct TransferIdRequest {
    pub transfer_id: String,
}

/// Parameters for [`RequestPayload::ImportWrite`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.ImportWriteRequest")]
pub struct ImportWriteRequest {
    pub transfer_id: String,
    /// Base64-encoded tar bytes.
    pub data: String,
}

/// Successful payload for the two `*Begin` transfer replies.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.TransferBeginResult")]
pub struct TransferBeginResult {
    pub transfer_id: String,
}

/// Successful payload for [`ResponsePayload::ExportRead`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.ExportChunk")]
pub struct ExportChunk {
    /// Base64-encoded tar bytes; may be non-empty on the final chunk.
    pub data: String,
    /// `true` ⇒ the export completed and its parked entry is gone.
    pub eof: bool,
}

/// Empty ack for the fire-and-forget-shaped replies.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.TransferAck")]
pub struct TransferAck {}

/// Successful payload for [`ResponsePayload::ImportEnd`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.ImportEndResult")]
pub struct ImportEndResult {
    pub bytes: u64,
}

/// Successful payload for [`ResponsePayload::LocalTransfer`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.LocalTransferResult")]
pub struct LocalTransferResult {
    pub bytes: u64,
}

/// Parameters for [`RequestPayload::Create`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.CreateRequest")]
pub struct CreateRequest {
    pub id: String,
    pub image: String,
    /// `[host, container]` bind-mount pairs.
    pub mounts: Vec<[String; 2]>,
    /// `[key, value]` environment pairs.
    pub env: Vec<[String; 2]>,
    pub cwd: String,
}

/// Parameters for [`RequestPayload::Delete`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.DeleteRequest")]
pub struct DeleteRequest {
    pub id: String,
}

/// Parameters for [`RequestPayload::LocalTransfer`] — both endpoints
/// live on the receiving host, so raw ids + paths are the whole
/// address.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.LocalTransferRequest")]
pub struct LocalTransferRequest {
    pub source_id: String,
    pub source_path: String,
    pub destination_id: String,
    pub destination_path: String,
}
