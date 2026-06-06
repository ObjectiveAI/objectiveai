//! `LogReference` for a per-message log file within a parent
//! [`AgentCompletionChunkLog`](super::AgentCompletionChunkLog).
//!
//! Response-side message envelopes are split by role on disk —
//! `messages/assistant/<id>_<idx>.json` vs
//! `messages/tool/<id>_<idx>.json` — so a bare `{type, path}` pointer
//! would force consumers to parse the role and index back out of the
//! path string. This reference carries both explicitly: after reading
//! the master chunk, `role` names the command family that reads the
//! sub-chunk (`… messages assistant get` vs `… messages tool get`) and
//! `index` is its message-index argument.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::logs::LogReferenceTag;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(
    rename = "agent.completions.response.streaming.LogReference"
)]
pub struct LogReference {
    #[serde(rename = "type")]
    pub r#type: LogReferenceTag,
    #[serde(skip_serializing_if = "String::is_empty")]
    #[schemars(extend("omitempty" = true))]
    pub path: String,
    /// The message's index within the completion — the message-index
    /// argument to the role-specific read commands.
    pub index: u64,
    /// Which role subdir the file lives in, i.e. which command family
    /// reads it.
    pub role: Role,
}

impl LogReference {
    pub fn new(path: String, index: u64, role: Role) -> Self {
        Self {
            r#type: LogReferenceTag::Reference,
            path,
            index,
            role,
        }
    }
}

/// The role of a referenced response-side message envelope — matches
/// the `role` tag serialized inside the referenced file
/// (`AssistantResponseChunkLog` / `ToolResponseLog`).
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    JsonSchema,
)]
#[serde(rename_all = "lowercase")]
#[schemars(
    rename = "agent.completions.response.streaming.Role"
)]
pub enum Role {
    Assistant,
    Tool,
}
