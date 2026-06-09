//! Streaming agent completion chunk type.

use crate::agent::completions::{message, response};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A chunk of a streaming agent completion response.
///
/// Multiple chunks are received via Server-Sent Events and can be
/// accumulated into a complete [`AgentCompletion`](response::unary::AgentCompletion)
/// using the [`push`](Self::push) method.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Default,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(
    rename = "agent.completions.response.streaming.AssistantResponseChunk"
)]
pub struct AssistantResponseChunk {
    pub role: response::AssistantRole,
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub index: u64,
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub created: u64,
    pub model: String,
    pub upstream_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub reasoning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub tool_calls: Option<Vec<message::AssistantToolCallDelta>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub content: Option<message::RichContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub refusal: Option<String>,
    pub finish_reason: Option<response::FinishReason>,
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
    /// Upstream usage for this assistant response (set by upstream clients).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub usage: Option<response::UpstreamUsage>,
    /// `message_queue_contents.id`s the API consumed to seed this
    /// turn's request. Stamped onto the first assistant chunk the
    /// API emits downstream — when set, the consumer owns these
    /// rows. Kinds are resolved CLI-side at write time (SQL CASE
    /// against `message_queue_contents.kind`), so the wire stays
    /// `i64`-only. Both `None` and `Some(empty)` are skipped on
    /// serialize.
    #[serde(default, skip_serializing_if = "request_message_ids_is_empty")]
    #[schemars(extend("omitempty" = true))]
    pub request_message_ids: Option<Vec<i64>>,
}

fn request_message_ids_is_empty(opt: &Option<Vec<i64>>) -> bool {
    opt.as_ref().map_or(true, |v| v.is_empty())
}

impl AssistantResponseChunk {
    /// Accumulates another chunk into this one.
    ///
    /// This is used to build up a complete response from streaming chunks.
    pub fn push(
        &mut self,
        AssistantResponseChunk {
            reasoning,
            tool_calls,
            content,
            refusal,
            finish_reason,
            logprobs,
            upstream_id,
            service_tier,
            system_fingerprint,
            provider,
            usage,
            request_message_ids,
            ..
        }: &AssistantResponseChunk,
    ) {
        response::util::push_option_string(&mut self.reasoning, reasoning);
        self.push_tool_calls(tool_calls);
        match (&mut self.content, content) {
            (Some(self_content), Some(other_content)) => {
                self_content.push(other_content);
            }
            (None, Some(other_content)) => {
                self.content = Some(other_content.clone());
            }
            _ => {}
        }
        response::util::push_option_string(&mut self.refusal, refusal);
        if self.finish_reason.is_none() {
            self.finish_reason = finish_reason.clone();
        }
        match (&mut self.logprobs, logprobs) {
            (Some(self_logprobs), Some(other_logprobs)) => {
                self_logprobs.push(other_logprobs);
            }
            (None, Some(other_logprobs)) => {
                self.logprobs = Some(other_logprobs.clone());
            }
            _ => {}
        }
        if self.upstream_id.is_empty() {
            self.upstream_id = upstream_id.clone();
        }
        if self.service_tier.is_none() {
            self.service_tier = service_tier.clone();
        }
        if self.system_fingerprint.is_none() {
            self.system_fingerprint = system_fingerprint.clone();
        }
        if self.provider.is_none() {
            self.provider = provider.clone();
        }
        match (&mut self.usage, usage) {
            (Some(self_usage), Some(other_usage)) => {
                self_usage.push(other_usage);
            }
            (None, Some(other_usage)) => {
                self.usage = Some(other_usage.clone());
            }
            _ => {}
        }
        match (
            &mut self.request_message_ids,
            request_message_ids.as_ref(),
        ) {
            (Some(existing), Some(incoming)) => {
                existing.extend(incoming.iter().copied());
            }
            (slot @ None, Some(incoming)) => {
                *slot = Some(incoming.clone());
            }
            _ => {}
        }
    }

    fn push_tool_calls(
        &mut self,
        other_tool_calls: &Option<Vec<message::AssistantToolCallDelta>>,
    ) {
        fn push_tool_call(
            tool_calls: &mut Vec<message::AssistantToolCallDelta>,
            other: &message::AssistantToolCallDelta,
        ) {
            fn find_tool_call(
                tool_calls: &mut Vec<message::AssistantToolCallDelta>,
                index: u64,
            ) -> Option<&mut message::AssistantToolCallDelta> {
                for tool_call in tool_calls {
                    if tool_call.index == index {
                        return Some(tool_call);
                    }
                }
                None
            }
            if let Some(tool_call) = find_tool_call(tool_calls, other.index) {
                tool_call.push(other);
            } else {
                tool_calls.push(other.clone());
            }
        }
        match (self.tool_calls.as_mut(), other_tool_calls) {
            (Some(self_tool_calls), Some(other_tool_calls)) => {
                for other_tool_call in other_tool_calls {
                    push_tool_call(self_tool_calls, other_tool_call);
                }
            }
            (None, Some(other_tool_calls)) => {
                self.tool_calls = Some(other_tool_calls.clone());
            }
            _ => {}
        }
    }

}
