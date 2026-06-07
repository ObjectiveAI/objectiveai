//! `AssistantResponseChunkLog` — postgres-log shape of
//! [`super::AssistantResponseChunk`].
//!
//! Field-for-field mirror with these swaps:
//!
//! - `content: Option<RichContent>` → `Option<RichContentLogRef>`
//! - `refusal: Option<String>` → `Option<LogRef>` (→ text)
//! - `reasoning: Option<String>` → `Option<LogRef>` (→ text)
//! - `tool_calls: Option<Vec<AssistantToolCallDelta>>` →
//!   `Option<Vec<AssistantToolCallDeltaLog>>` (the delta's `arguments`
//!   string is extracted to a text ref)
//! - `logprobs` stays inline — structured, not a media type.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::agent::completions::message::AssistantToolCallType;
use crate::agent::completions::response;
use crate::logs::{LogRef, RichContentLogRef};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(
    rename = "agent.completions.response.streaming.AssistantResponseChunkLog"
)]
pub struct AssistantResponseChunkLog {
    pub role: response::AssistantRole,
    pub index: u64,
    pub created: u64,
    pub model: String,
    pub upstream_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub reasoning: Option<LogRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub tool_calls: Option<Vec<AssistantToolCallDeltaLog>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub content: Option<RichContentLogRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub refusal: Option<LogRef>,
    pub finish_reason: Option<response::FinishReason>,
    /// Logprobs stay inline — structured payload, not a media type.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub logprobs: Option<response::Logprobs>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub service_tier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub system_fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub usage: Option<response::UpstreamUsage>,
}

/// Streaming tool-call delta with the streaming-JSON `arguments`
/// extracted to a `text` ref.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(
    rename = "agent.completions.response.streaming.AssistantToolCallDeltaLog"
)]
pub struct AssistantToolCallDeltaLog {
    pub index: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub r#type: Option<AssistantToolCallType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub function: Option<AssistantToolCallFunctionDeltaLog>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(
    rename = "agent.completions.response.streaming.AssistantToolCallFunctionDeltaLog"
)]
pub struct AssistantToolCallFunctionDeltaLog {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub name: Option<String>,
    /// Streaming JSON-args delta → `text` table ref.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub arguments: Option<LogRef>,
}
