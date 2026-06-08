//! Assistant response type for unary agent completions.

use crate::agent::completions::{message, response};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An assistant response in a unary agent completion.
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, Default, JsonSchema,
)]
#[schemars(rename = "agent.completions.response.unary.AssistantResponse")]
pub struct AssistantResponse {
    pub role: response::AssistantRole,
    pub index: u64,
    pub created: u64,
    pub model: String,
    pub upstream_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub reasoning: Option<String>,
    pub tool_calls: Option<Vec<message::AssistantToolCall>>,
    pub content: Option<message::RichContent>,
    pub refusal: Option<String>,
    pub finish_reason: response::FinishReason,
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
    /// Upstream usage for this assistant response (set by upstream clients).
    pub usage: response::UpstreamUsage,
}

impl From<response::streaming::AssistantResponseChunk> for AssistantResponse {
    fn from(
        response::streaming::AssistantResponseChunk {
            role,
            index,
            created,
            model,
            upstream_id,
            reasoning,
            tool_calls,
            content,
            refusal,
            finish_reason,
            logprobs,
            service_tier,
            system_fingerprint,
            provider,
            usage,
            // `request_message_ids` is a streaming-side signal —
            // the unary response shape ignores it.
            request_message_ids: _,
        }: response::streaming::AssistantResponseChunk,
    ) -> Self {
        Self {
            role,
            index,
            created,
            model,
            upstream_id,
            reasoning,
            tool_calls: tool_calls.map(|tool_calls| {
                tool_calls.into_iter().map(Into::into).collect()
            }),
            content,
            refusal,
            finish_reason: finish_reason.unwrap_or_default(),
            logprobs,
            service_tier,
            system_fingerprint,
            provider,
            usage: usage.unwrap_or_default(),
        }
    }
}
