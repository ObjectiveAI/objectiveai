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
/// (alongside the typed params via `#[serde(flatten)]`). Non-MCP
/// variants (`ReadMessageQueue` / `ClearMessageQueue`) don't carry
/// `mcp_kind` at all — they hit the CLI's own local state and never
/// route to an upstream MCP server. Use [`Payload::mcp_kind`] to
/// retrieve it generically.
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

    /// Read the CLI's local message queue (`prompts.sqlite`) for a
    /// given agent hierarchy. Non-MCP — no `mcp_kind`. Non-destructive;
    /// pair with [`Payload::ClearMessageQueue`] to release rows
    /// after the caller has consumed them.
    #[schemars(title = "ReadMessageQueue")]
    ReadMessageQueue(ReadMessageQueueRequest),

    /// Delete a set of message-queue rows by id. Non-MCP — no
    /// `mcp_kind`. Unknown ids are silently ignored.
    #[schemars(title = "ClearMessageQueue")]
    ClearMessageQueue(ClearMessageQueueRequest),
}

impl Payload {
    /// Which CLI-hosted MCP server this payload targets. `Some` for
    /// the MCP-routed variants; `None` for `ReadMessageQueue` and
    /// `ClearMessageQueue` which hit the CLI's own local state.
    pub fn mcp_kind(&self) -> Option<super::super::McpKind> {
        match self {
            Payload::Initialize { mcp_kind, .. }
            | Payload::ToolsList { mcp_kind, .. }
            | Payload::ToolsCall { mcp_kind, .. }
            | Payload::ResourcesList { mcp_kind, .. }
            | Payload::ResourcesRead { mcp_kind, .. }
            | Payload::SessionTerminate { mcp_kind } => Some(mcp_kind.clone()),
            Payload::ReadMessageQueue(_) | Payload::ClearMessageQueue(_) => None,
        }
    }
}

/// Parameters for [`Payload::ReadMessageQueue`].
///
/// Three-rule predicate (matches `drain_for_message` +
/// `drain_for_spawn` combined):
/// 1. Direct hit — `prompts.agent_instance_hierarchy =
///    agent_instance_hierarchy`.
/// 2. BOUND-tag hit — `prompts.agent_tag` resolves to a tag whose
///    `tags.agent_instance_hierarchy = agent_instance_hierarchy`.
/// 3. PENDING-tag hit — `prompts.agent_tag` resolves to a tag in
///    PENDING state whose
///    `(parent_agent_instance_hierarchy, agent_full_id)` matches the
///    fields below. Lets the API drain rows that were enqueued
///    against a tag whose spawn this agent is.
///
/// Returns rows oldest-first (`prompts.id ASC`, which also matches
/// `prompts.enqueued_at` ascending due to AUTOINCREMENT). Pair with
/// [`ClearMessageQueueRequest`] (same scope fields) after processing
/// to release the rows; rows left behind remain visible to the next
/// read.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_request.ReadMessageQueueRequest")]
pub struct ReadMessageQueueRequest {
    pub agent_instance_hierarchy: String,
    /// Lineage prefix used by rule 3 to find PENDING tags. `None`
    /// for rootless agents — the tag's
    /// `parent_agent_instance_hierarchy` is stored as `""` for those
    /// (see `tags::upgrade`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub parent_agent_instance_hierarchy: Option<String>,
    /// Agent full id used by rule 3 to find PENDING tags.
    pub agent_full_id: String,
}

/// Parameters for [`Payload::ClearMessageQueue`].
///
/// Scope fields mirror [`ReadMessageQueueRequest`] — the same
/// three-rule predicate gates which ids may be cleared. Ids outside
/// the scope are silently absorbed; this protects against an API
/// caller mis-addressing a row that belongs to a different agent.
///
/// `ON DELETE CASCADE` on `prompt_contents.prompt_id` sweeps the
/// per-kind content rows. Empty `ids` is a no-op. Unknown ids are
/// silently ignored — the API may have raced another reader.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "client_objectiveai_mcp.server_request.ClearMessageQueueRequest")]
pub struct ClearMessageQueueRequest {
    pub agent_instance_hierarchy: String,
    /// Lineage prefix; see [`ReadMessageQueueRequest`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub parent_agent_instance_hierarchy: Option<String>,
    /// Agent full id; see [`ReadMessageQueueRequest`].
    pub agent_full_id: String,
    pub ids: Vec<i64>,
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
