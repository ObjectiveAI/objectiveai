//! `AgentCompletionChunkLog` — postgres-log shape of
//! [`super::AgentCompletionChunk`].
//!
//! Field-for-field mirror with two swaps:
//!
//! - `messages: Vec<MessageChunk>` → `Vec<MessageChunkLog>` — each
//!   per-role message chunk is stripped (assistant deltas via
//!   `AssistantResponseChunkLog`; tool responses via `ToolResponseLog`).
//!   Inlined, not a cross-table ref, since these chunks belong to this
//!   agent-completion response row and never recur elsewhere.
//! - `continuation: Option<String>` → `Option<LogRef>` (→ text).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::agent;
use crate::agent::completions::response;
use crate::logs::LogRef;

use super::AssistantResponseChunkLog;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(
    rename = "agent.completions.response.streaming.AgentCompletionChunkLog"
)]
pub struct AgentCompletionChunkLog {
    pub id: String,
    pub agent_instance_hierarchy: String,
    pub agent_id: String,
    pub agent_full_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent_remote: Option<crate::RemotePath>,
    pub created: u64,
    pub messages: Vec<MessageChunkLog>,
    pub object: response::streaming::Object,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub usage: Option<response::Usage>,
    pub upstream: agent::Upstream,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub error: Option<crate::error::ResponseError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub continuation: Option<LogRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub messages_queued: Option<bool>,
}

/// Stripped per-role message chunk variant — mirrors the wire-side
/// [`super::MessageChunk`] enum. Untagged on the wire: each inner
/// payload carries its own discriminating `role` field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(
    rename = "agent.completions.response.streaming.MessageChunkLog"
)]
pub enum MessageChunkLog {
    #[schemars(title = "Assistant")]
    Assistant(AssistantResponseChunkLog),
    #[schemars(title = "Tool")]
    Tool(crate::agent::completions::response::ToolResponseLog),
}
