//! Streaming agent completion chunk type.

use crate::agent::completions::response;
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
    rename = "agent.completions.response.streaming.AgentCompletionChunk"
)]
pub struct AgentCompletionChunk {
    pub id: String,
    /// Full agent instance hierarchy for this completion's slot —
    /// `{ctx lineage}/{agent_full_id}-{response_id}`, or the fixed
    /// continuation value on resume. Same on every chunk of a slot.
    pub agent_instance_hierarchy: String,
    /// Leaf agent id of the slot that produced this chunk. For the
    /// primary attempt this is the primary agent's id; on fallback it
    /// is the fallback agent's id. Same on every chunk of a slot.
    pub agent_id: String,
    /// WF-level id: concatenation of the primary agent's id with all
    /// fallback ids (see `InlineAgentWithFallbacks::full_id`). Same
    /// for every slot in the same WF request.
    pub agent_full_id: String,
    /// `RemotePath` the WF was fetched from. `None` when the WF was
    /// supplied inline. Same for every slot in the same WF request.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub agent_remote: Option<crate::RemotePath>,
    /// The resolved inline WF definition. Populated ONLY on the FIRST
    /// chunk of a completion — but ALWAYS on the first chunk, remote
    /// or not (pair with [`Self::agent_remote`] to tell which source
    /// it came from). [`Self::push`] keeps the first value.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(default)]
    pub agent_inline: Option<crate::agent::InlineAgentWithFallbacks>,
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub created: u64,
    pub messages: Vec<super::MessageChunk>,
    /// The object type (always "agent.completion.chunk").
    pub object: super::Object,
    /// Token usage (only present in the final chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub usage: Option<response::Usage>,
    /// Upstream provider
    pub upstream: crate::agent::Upstream,
    /// Error details if this completion failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub error: Option<crate::error::ResponseError>,
    /// Continuation state for multi-turn conversations (only present in the final chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub continuation: Option<String>,
    /// `true` when the MCP proxy holds queued messages that were not
    /// delivered to the agent via a tool response on this turn. Only
    /// set when `continuation` is also set — the caller acts on it by
    /// issuing the continuation. Absent when nothing is queued, when
    /// there is no continuation to act on, or when the peek failed
    /// (the failure is surfaced via `error`).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub messages_queued: Option<bool>,
}

impl AgentCompletionChunk {
    /// Accumulates another chunk into this one.
    ///
    /// This is used to build up a complete response from streaming chunks.
    pub fn push(
        &mut self,
        AgentCompletionChunk {
            messages,
            usage,
            error,
            continuation,
            messages_queued,
            agent_inline,
            ..
        }: &AgentCompletionChunk,
    ) {
        self.push_messages(messages);
        // First chunk wins: `agent_inline` rides only the completion's
        // first chunk, so the accumulator never overwrites it.
        if self.agent_inline.is_none() {
            if let Some(agent_inline) = agent_inline {
                self.agent_inline = Some(agent_inline.clone());
            }
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
        if let Some(error) = error {
            self.error = Some(error.clone());
        }
        if let Some(continuation) = continuation {
            self.continuation = Some(continuation.clone());
        }
        if let Some(mq) = messages_queued {
            self.messages_queued = Some(*mq);
        }
    }

    fn push_messages(&mut self, other_choices: &[super::MessageChunk]) {
        fn push_message(
            messages: &mut Vec<super::MessageChunk>,
            other: &super::MessageChunk,
        ) {
            fn find_message(
                messages: &mut Vec<super::MessageChunk>,
                index: u64,
            ) -> Option<&mut super::MessageChunk> {
                for message in messages {
                    if message.index() == index {
                        return Some(message);
                    }
                }
                None
            }
            if let Some(message) = find_message(messages, other.index()) {
                message.push(other);
            } else {
                messages.push(other.clone());
            }
        }
        for other_message in other_choices {
            push_message(&mut self.messages, other_message);
        }
    }
}
