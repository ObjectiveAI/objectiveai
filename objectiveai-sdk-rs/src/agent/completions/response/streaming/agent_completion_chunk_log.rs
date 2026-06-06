//! On-disk shape of an `AgentCompletionChunk` log file.
//!
//! Mirrors [`super::AgentCompletionChunk`] field-for-field, with
//! two type swaps:
//!
//! - `messages: Vec<MessageChunk>` →
//!   `Vec<message_log_reference::LogReference>` since each message is
//!   extracted to its own role-subdir file; the reference carries the
//!   message's `index` and `role` so consumers know which
//!   role-specific command reads it without parsing the path.
//! - `continuation: Option<String>` → `Option<LogReference>` since
//!   the continuation token is extracted to its own file.
//!
//! Field declaration order matches the wire chunk so today's
//! `serde_json::to_value(&shell)` byte-shape is preserved.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::agent;
use crate::agent::completions::response;
use crate::logs::LogReference;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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
    pub messages: Vec<super::message_log_reference::LogReference>,
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
    pub continuation: Option<LogReference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub messages_queued: Option<bool>,
}
