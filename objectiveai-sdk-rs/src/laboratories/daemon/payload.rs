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
    /// Whether the daemon currently holds ≥1 live filetree subscriber
    /// for the envelope's laboratory (edge-triggered: sent on the
    /// 0→1 and 1→0 transitions, and replayed as `on: true` when the
    /// host reconnects while subscribers exist). Drives the host's
    /// container lifecycle: `on` lazily starts the container (and its
    /// filetree pump); `off` makes it a stop candidate once the lab
    /// also has no MCP connections. Defaults to off — a host that
    /// never receives this treats the lab as unwatched.
    #[schemars(title = "Filetree")]
    Filetree(FiletreeRequest),
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
    /// Create an EPHEMERAL agent laboratory AND establish its single
    /// MCP connection, atomically: ensure the content-addressed agent
    /// image, `rm -f` any stale container for the id, create + start
    /// the container, connect using THIS request's envelope headers
    /// (they carry the FULL originating header set — the agent
    /// arguments the in-container MCP requires; unlike `Create`, which
    /// rides with an empty map). Succeeds only when BOTH the container
    /// and the connection exist; the reply carries identity AND the
    /// initialize result. Exactly ONE connection, ever — the
    /// laboratory's lifetime IS that connection's, and the container
    /// evaporates (`rm -f`, zero grace) the moment it ends. Host-level:
    /// no envelope `laboratory_id` — the HOST derives the id
    /// (`{derived_id}-{response_id}`, see
    /// [`crate::agent::laboratories::ephemeral_id`]).
    ///
    /// NOTE for the daemon phase: the image ensure runs a pull/build
    /// inline, which can exceed the standard RPC forward timeout —
    /// classify this op with the timeout-free transfer family.
    #[schemars(title = "AgentEphemeralCreate")]
    AgentEphemeralCreate(AgentEphemeralCreateRequest),
    /// The PLUGIN twin of [`RequestPayload::AgentEphemeralCreate`]:
    /// ensure the plugin image (git fetch at the version tag + `podman
    /// build` — timeout-free family, same note as above), then the
    /// same atomic create + start + connect with this request's
    /// headers. The HOST derives the id
    /// (`oai-plugin-{owner}-{name}-{version}-{response_id}`).
    #[schemars(title = "PluginEphemeralCreate")]
    PluginEphemeralCreate(PluginEphemeralCreateRequest),
    /// Delete a laboratory from this host (podman rm). Host-level.
    #[schemars(title = "Delete")]
    Delete(DeleteRequest),
    /// Transfer a path between TWO laboratories that both live on
    /// THIS host: the host pipes the source's export stream straight
    /// into the destination's import — no chunk staging anywhere.
    /// Host-level: it addresses two labs, never the envelope's one.
    #[schemars(title = "LocalTransfer")]
    LocalTransfer(LocalTransferRequest),
    /// Build a plugin's VIEWER extension: fetch the repo at the
    /// version's git tag, run the baked builder image over the
    /// checkout in a BUILD laboratory, and park the resulting
    /// `bundle.tar.gz` for [`RequestPayload::BuildRead`]. Runs to
    /// completion — the container is created, waited on, and removed
    /// within this one request (timeout-free family: a cold build
    /// pulls a base image and installs a dependency tree). Host-level:
    /// the HOST derives the laboratory id.
    #[schemars(title = "BuildCreate")]
    BuildCreate(BuildCreateRequest),
    /// Drain the next chunk of a parked build artifact — the
    /// [`RequestPayload::ExportRead`] shape for a host-side FILE
    /// rather than a container's export stream (a build container
    /// serves no HTTP: it has no injected MCP binary and is gone by
    /// the time this runs). Host-level.
    #[schemars(title = "BuildRead")]
    BuildRead(TransferIdRequest),
    /// Discard a parked build artifact without draining it.
    /// Host-level.
    #[schemars(title = "BuildAbort")]
    BuildAbort(TransferIdRequest),
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
    #[schemars(title = "Filetree")]
    Filetree(JsonRpcResult<TransferAck>),
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
    #[schemars(title = "AgentEphemeralCreate")]
    AgentEphemeralCreate(JsonRpcResult<EphemeralCreated>),
    #[schemars(title = "PluginEphemeralCreate")]
    PluginEphemeralCreate(JsonRpcResult<EphemeralCreated>),
    #[schemars(title = "Delete")]
    Delete(JsonRpcResult<TransferAck>),
    #[schemars(title = "LocalTransfer")]
    LocalTransfer(JsonRpcResult<LocalTransferResult>),
    #[schemars(title = "BuildCreate")]
    BuildCreate(JsonRpcResult<BuildCreated>),
    #[schemars(title = "BuildRead")]
    BuildRead(JsonRpcResult<ExportChunk>),
    #[schemars(title = "BuildAbort")]
    BuildAbort(JsonRpcResult<TransferAck>),
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

/// Parameters for [`RequestPayload::Filetree`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.FiletreeRequest")]
pub struct FiletreeRequest {
    /// `true` ⇒ the daemon holds ≥1 live filetree subscriber for this
    /// laboratory; `false` ⇒ it holds none.
    pub on: bool,
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
    /// RAW tar bytes. Never in the JSON header — rides OUT OF BAND in
    /// the binary wire frame (`crate::binary_frame`); serde skips it.
    #[serde(skip)]
    pub data: Vec<u8>,
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
    /// RAW tar bytes; may be non-empty on the final chunk. Never in
    /// the JSON header — rides OUT OF BAND in the binary wire frame
    /// (`crate::binary_frame`); serde skips it.
    #[serde(skip)]
    pub data: Vec<u8>,
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
    /// The base image, always split — the joined reference string is
    /// constructed only inside the host's podman internals.
    pub image: crate::laboratories::LaboratoryImage,
    /// `[host, container]` bind-mount pairs.
    pub mounts: Vec<[String; 2]>,
    /// `[key, value]` environment pairs.
    pub env: Vec<[String; 2]>,
    pub cwd: String,
    /// For agent laboratories: the full id of the agent the
    /// laboratory derives from. `Some` ⇔ the id is under the reserved
    /// `oai-agent-` prefix (the host enforces both directions).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent_full_id: Option<String>,
}

/// Parameters for [`RequestPayload::AgentEphemeralCreate`].
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.AgentEphemeralCreateRequest")]
pub struct AgentEphemeralCreateRequest {
    /// The agent-completion response id this laboratory serves — bare
    /// base62, unique per completion. Becomes the laboratory id's
    /// suffix and the container label's `response_id`.
    pub response_id: String,
    /// The full id of the agent the laboratory derives from.
    pub agent_full_id: String,
    /// The agent-embedded laboratory spec (image, env, cwd) — the
    /// host re-derives the content-addressed image key from
    /// `(agent_full_id, laboratory)`.
    pub laboratory: crate::agent::Laboratory,
}

/// Parameters for [`RequestPayload::PluginEphemeralCreate`] — the
/// plugin's coordinate trio (`agent::plugin::prepare` already
/// lowercases owner/name; the host re-lowercases them defensively —
/// the version passes through untouched: it is required `v`-prefixed
/// at the declaration layer and git tags are case-sensitive) plus the
/// response id the laboratory serves.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.PluginEphemeralCreateRequest")]
pub struct PluginEphemeralCreateRequest {
    /// The agent-completion response id this laboratory serves.
    pub response_id: String,
    /// GitHub `<owner>` segment.
    pub owner: String,
    /// Repository segment.
    pub name: String,
    /// Plugin version — IS the repo's git tag (Go-modules style,
    /// `v`-prefixed), byte-for-byte; the tag must exist.
    pub version: String,
    /// The plugin's Postgres compartment ROLE, resolved (and
    /// provisioned) daemon-side before the create is forwarded. The
    /// host builds `OBJECTIVEAI_POSTGRES_URL` from this + its own
    /// tunnel-listener address (it never sees the daemon's own DB
    /// address).
    pub db_role: String,
    /// The compartment role's password (random, stored daemon-side).
    pub db_password: String,
    /// The application database name.
    pub db_database: String,
}

/// Successful payload for BOTH ephemeral-create replies: the created
/// laboratory's identity (`response_id` Some, `running` true) AND its
/// single MCP connection's initialize result — one round trip, both
/// or neither.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.EphemeralCreated")]
pub struct EphemeralCreated {
    pub identify: super::Identify,
    pub reply: InitializeReply,
}

/// Parameters for [`RequestPayload::BuildCreate`] — the plugin's
/// coordinate trio, exactly as [`PluginEphemeralCreateRequest`] takes
/// it (owner/name lowercased defensively host-side; the version passes
/// through untouched — it IS the git tag, byte-for-byte).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.BuildCreateRequest")]
pub struct BuildCreateRequest {
    /// GitHub `<owner>` segment.
    pub owner: String,
    /// Repository segment.
    pub name: String,
    /// Plugin version — IS the repo's git tag; the tag must exist.
    pub version: String,
}

/// Successful payload for [`ResponsePayload::BuildCreate`]: the build
/// SUCCEEDED and its artifact is parked. Nothing is streamed before
/// this lands — a failed build never produces a truncated archive.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.daemon.BuildCreated")]
pub struct BuildCreated {
    /// The commit the version's tag resolved to — the daemon's
    /// `X-OBJECTIVEAI-SHA`.
    pub commit_sha: String,
    /// Drain handle for [`RequestPayload::BuildRead`].
    pub transfer_id: String,
    /// The artifact's total size in bytes.
    pub bytes: u64,
}

/// The JSON-RPC error code [`RequestPayload::BuildCreate`] answers
/// with when the plugin's git TAG does not exist — the one build
/// failure that is the caller's fault rather than the plugin's, and
/// the daemon's `404`. Every other failure uses the generic internal
/// code and becomes a `500`. A code (not a message substring) carries
/// it: the old route sniffed `"not found in"` out of an error string,
/// which is exactly the kind of coupling this replaces.
pub const BUILD_TAG_NOT_FOUND_CODE: i64 = -32004;

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
