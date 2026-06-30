use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Tagged union of every JSON-RPC request the API forwards to the
/// client over the reverse-attach WS. Variant names follow the same
/// snake_case convention `client_request::Payload` uses; the
/// `serde(tag = "type")` discriminator pairs with
/// [`super::super::server_response::Payload`] by name.
///
/// MCP-routed variants carry `mcp_kind` directly on the variant
/// (alongside the typed params via `#[serde(flatten)]`). The non-MCP
/// `ReadMessageQueue` variant doesn't carry `mcp_kind` at all — it
/// hits the CLI's own local state and never routes to an upstream
/// MCP server. Use [`Payload::mcp_kind`] to retrieve it generically.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_request.Payload")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Payload {
    /// POST `initialize`. The proxy's `protocolVersion` doesn't ride
    /// across this hop — the API discards it on the way in and
    /// substitutes its own `canonical_initialize_result` on the way
    /// out. The variant carries the plugin arguments the CLI needs at
    /// dial time (parsed by the API off the URL query string).
    #[schemars(title = "Initialize")]
    Initialize {
        mcp_kind: super::super::McpKind,
        #[serde(flatten)]
        params: InitializeRequest,
    },

    /// POST `tools/list`.
    #[schemars(title = "ToolsList")]
    ToolsList {
        mcp_kind: super::super::McpKind,
        #[serde(flatten)]
        params: crate::mcp::tool::ListToolsRequest,
    },

    /// POST `tools/call`.
    #[schemars(title = "ToolsCall")]
    ToolsCall {
        mcp_kind: super::super::McpKind,
        #[serde(flatten)]
        params: crate::mcp::tool::CallToolRequestParams,
    },

    /// POST `resources/list`.
    #[schemars(title = "ResourcesList")]
    ResourcesList {
        mcp_kind: super::super::McpKind,
        #[serde(flatten)]
        params: crate::mcp::resource::ListResourcesRequest,
    },

    /// POST `resources/read`.
    #[schemars(title = "ResourcesRead")]
    ResourcesRead {
        mcp_kind: super::super::McpKind,
        #[serde(flatten)]
        params: crate::mcp::resource::ReadResourceRequestParams,
    },

    /// `DELETE` on the routed MCP URL — the proxy closing the
    /// session. No body beyond `mcp_kind`.
    #[schemars(title = "SessionTerminate")]
    SessionTerminate { mcp_kind: super::super::McpKind },

    /// Read the CLI's local message queue for a given agent
    /// hierarchy. Non-MCP — no `mcp_kind`. Non-destructive: the
    /// API stamps the consumed ids onto the first
    /// `AssistantResponseChunk.request_message_ids` it emits so
    /// the downstream consumer owns row deletion; no separate
    /// release RPC.
    #[schemars(title = "ReadMessageQueue")]
    ReadMessageQueue(ReadMessageQueueRequest),

    /// Resolve a `Client` remote from the client's own local storage
    /// (agent / swarm / function / profile). Non-MCP — no `mcp_kind`.
    #[schemars(title = "Retrieve")]
    Retrieve(super::super::retrieve::Request),

    /// Forceful bulk teardown of every upstream connection for one
    /// objectiveai response id. Non-MCP — no `mcp_kind` (it spans all
    /// kinds for that response id). The CLI removes the whole
    /// response-id bucket from its connection registry, which drops
    /// every connection and kills every plugin subprocess under it.
    /// Distinct from `SessionTerminate` (graceful per-`mcp_kind` MCP
    /// `DELETE`); `Drop` is drop = kill.
    #[schemars(title = "Drop")]
    Drop(DropRequest),

    /// Copy a file/folder from one laboratory to another (both on this
    /// conduit). Non-MCP — it spans TWO laboratories, so it carries their
    /// ids directly rather than a single `mcp_kind`. The conduit streams
    /// the source laboratory's `/export` straight into the destination's
    /// `/import` (a tar splice, no intermediate buffering).
    #[schemars(title = "LaboratoryTransfer")]
    LaboratoryTransfer(LaboratoryTransferRequest),
}

impl Payload {
    /// Which CLI-hosted MCP server this payload targets. `Some` for
    /// the MCP-routed variants; `None` for `ReadMessageQueue` which
    /// hits the CLI's own local state.
    pub fn mcp_kind(&self) -> Option<super::super::McpKind> {
        match self {
            Payload::Initialize { mcp_kind, .. }
            | Payload::ToolsList { mcp_kind, .. }
            | Payload::ToolsCall { mcp_kind, .. }
            | Payload::ResourcesList { mcp_kind, .. }
            | Payload::ResourcesRead { mcp_kind, .. }
            | Payload::SessionTerminate { mcp_kind } => Some(mcp_kind.clone()),
            Payload::ReadMessageQueue(_)
            | Payload::Retrieve(_)
            | Payload::Drop(_)
            | Payload::LaboratoryTransfer(_) => None,
        }
    }
}

/// Parameters for [`Payload::ReadMessageQueue`].
///
/// Two-rule predicate (now that PENDING is gone):
/// 1. Direct hit — `message_queue.agent_instance_hierarchy =
///    agent_instance_hierarchy`.
/// 2. BOUND-tag hit — `message_queue.agent_tag` resolves to a tag
///    whose `tags.agent_instance_hierarchy = agent_instance_hierarchy`.
///
/// The conduit-side spawn-with-tag flow pre-fires the tag-group
/// upgrade ahead of every read, so any tags sharing the spawn's
/// group become BOUND before the EXISTS-check runs and feed
/// straight into rule 2.
///
/// Returns rows oldest-first (`message_queue.id ASC`, which also
/// matches `message_queue.enqueued_at` ascending due to
/// AUTOINCREMENT). Row deletion is the downstream consumer's job:
/// the API stamps the consumed ids onto the first emitted
/// `AssistantResponseChunk.request_message_ids` so the consumer
/// knows which rows it owns.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_request.ReadMessageQueueRequest")]
pub struct ReadMessageQueueRequest {
    pub agent_instance_hierarchy: String,
}

/// Parameters for [`Payload::Drop`]. The objectiveai response id whose
/// entire upstream-connection bucket the CLI should tear down. The id
/// rides in the payload (not a header) because `Drop` is a control
/// message identified solely by its argument.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_request.DropRequest")]
pub struct DropRequest {
    pub response_id: String,
}

/// Parameters for [`Payload::LaboratoryTransfer`]. Copy `source_path` from
/// the `source_id` laboratory into `dest_path` of the `dest_id` laboratory
/// (cp-style: a directory `source_path` lands as `dest_path/<basename>`).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_request.LaboratoryTransferRequest")]
pub struct LaboratoryTransferRequest {
    pub source_id: String,
    pub dest_id: String,
    pub source_path: String,
    pub dest_path: String,
}

/// Parameters for [`Payload::Initialize`].
///
/// Carries plugin arguments lifted off the inbound URL's query
/// string (`?key=value&flag` → `{"key": Some("value"), "flag": None}`).
/// Empty for [`super::super::McpKind::ObjectiveAi`] (the primary
/// upstream takes no args).
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_request.InitializeRequest")]
pub struct InitializeRequest {
    /// Plugin arguments the CLI passes through to
    /// `<plugin> mcp <mcp_name> begin --<key> [value]`. `None` value
    /// means presence-only flag (`--key`); `Some(v)` means `--key v`.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    #[schemars(extend("omitempty" = true))]
    pub args: IndexMap<String, Option<String>>,
}
