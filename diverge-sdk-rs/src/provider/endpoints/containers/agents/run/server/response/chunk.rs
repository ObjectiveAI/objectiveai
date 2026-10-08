//! The agentic loop chunk — the unit of a streaming response.

use rmcp::model::MetaObject;
use serde::{Deserialize, Serialize};

use super::{
    AssistantAudioContentChunk, AssistantImageContentChunk,
    AssistantReasoningChunk, AssistantRefusalChunk,
    AssistantTextContentChunk, AssistantToolCallChunk, NotificationChunk,
    ToolResponseChunk, UsageChunk, UserAudioContentChunk, UserImageContentChunk, UserResourceChunk,
    UserResourceLinkChunk, UserTextContentChunk,
};
use crate::shared::containers::request::Image;
use crate::shared::mcp::{Who, attest};

/// One chunk of a streaming agentic loop.
///
/// Each chunk is ONE event, not a struct of mostly-absent optionals.
/// A consumer learns what happened by matching, rather than by
/// inspecting which fields happen to be set.
///
/// **Untagged, discriminated by payload.** serde adds no tag of its
/// own; instead every variant's payload carries a `type` field whose
/// value no other variant can produce — each `type` is its own
/// single-variant enum, so a wrong value fails to deserialize instead
/// of arriving as data nobody checks. So the wire shape is the event
/// itself rather than a wrapper around one, and deserialization is
/// still unambiguous — the `type` constants do the work a tag would,
/// without a level of nesting.
///
/// That also means variant order here is not load-bearing. Untagged
/// deserialization takes the first variant that matches, and with
/// distinct `type` constants at most one ever can.
///
/// # Sub-agents
///
/// An upstream that delegates to sub-agents produces chunks on more
/// than one thread. The six assistant chunks and the tool response
/// carry `parent_tool_call_id` for that: absent, the chunk is the
/// main thread's; present, it is the id of the tool call whose
/// sub-agent produced it, so a caller sees a sub-agent's work as
/// children of the call that made it — its own calls answered by
/// its own responses, attributed alike. A nested sub-agent names
/// its IMMEDIATE spawner, so depth is a chain of ids. The other
/// chunks are the run's, not any thread's, and carry nothing.
///
/// # User parts
///
/// A message of the caller's is not one chunk but its parts: one
/// chunk per content block, in the message's order, each carrying
/// the message's `key` — the five user kinds, one per kind MCP's
/// content has, mirroring the assistant's but for a key in place of
/// a thread and whole in place of a delta. A message is its run of
/// parts under one key; the first chunks of every run are the parts
/// of the message it started on; see [`user_parts`](super::user_parts).
///
/// # Who spoke, under `_meta`
///
/// Every kind of chunk has a `_meta` at its top level — MCP's own on
/// the flattened content and results, this crate's on the rest — and
/// the daemon sets the keys of [`shared::mcp`](crate::shared::mcp)
/// on each as it keeps the stream, by [`attest`](Self::attest): the
/// speaking agent's image, template and index. The proxy and the
/// provider relay each chunk as the agent's server produced it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AgenticLoopChunk {
    /// The model's reasoning. See [`AssistantReasoningChunk`].
    AssistantReasoning(AssistantReasoningChunk),
    /// Text from the model. See [`AssistantTextContentChunk`].
    AssistantTextContent(AssistantTextContentChunk),
    /// An image from the model. See [`AssistantImageContentChunk`].
    AssistantImageContent(AssistantImageContentChunk),
    /// Audio from the model. See [`AssistantAudioContentChunk`].
    AssistantAudioContent(AssistantAudioContentChunk),
    /// The model calling a tool. See [`AssistantToolCallChunk`].
    AssistantToolCall(AssistantToolCallChunk),
    /// The model declining. See [`AssistantRefusalChunk`].
    AssistantRefusal(AssistantRefusalChunk),
    /// A tool's result. See [`ToolResponseChunk`].
    ToolResponse(ToolResponseChunk),
    /// Text of a message of the caller's. See [`UserTextContentChunk`].
    UserTextContent(UserTextContentChunk),
    /// An image of a message of the caller's. See
    /// [`UserImageContentChunk`].
    UserImageContent(UserImageContentChunk),
    /// Audio of a message of the caller's. See
    /// [`UserAudioContentChunk`].
    UserAudioContent(UserAudioContentChunk),
    /// An embedded resource of a message of the caller's. See
    /// [`UserResourceChunk`].
    UserResource(UserResourceChunk),
    /// A resource link of a message of the caller's. See
    /// [`UserResourceLinkChunk`].
    UserResourceLink(UserResourceLinkChunk),
    /// Token usage so far. See [`UsageChunk`].
    Usage(UsageChunk),
    /// Something about the run itself. See [`NotificationChunk`].
    Notification(NotificationChunk),
}

impl AgenticLoopChunk {
    /// Set the keys of [`shared::mcp`](crate::shared::mcp) on this
    /// chunk's `_meta`, whichever kind it is — made when the chunk
    /// carried none: the speaking agent's image and key, in place of
    /// whatever was under those keys and beside everything else.
    pub fn attest(&mut self, image: Option<&Image>, who: Who<'_>) {
        let meta: &mut MetaObject = match self {
            AgenticLoopChunk::AssistantReasoning(chunk) => chunk.inner.meta.get_or_insert_default(),
            AgenticLoopChunk::AssistantTextContent(chunk) => chunk.inner.meta.get_or_insert_default(),
            AgenticLoopChunk::AssistantImageContent(chunk) => chunk.inner.meta.get_or_insert_default(),
            AgenticLoopChunk::AssistantAudioContent(chunk) => chunk.inner.meta.get_or_insert_default(),
            AgenticLoopChunk::AssistantToolCall(chunk) => &mut **chunk.meta.get_or_insert_default(),
            AgenticLoopChunk::AssistantRefusal(chunk) => chunk.inner.meta.get_or_insert_default(),
            AgenticLoopChunk::ToolResponse(chunk) => chunk.inner.meta.get_or_insert_default(),
            AgenticLoopChunk::UserTextContent(chunk) => chunk.inner.meta.get_or_insert_default(),
            AgenticLoopChunk::UserImageContent(chunk) => chunk.inner.meta.get_or_insert_default(),
            AgenticLoopChunk::UserAudioContent(chunk) => chunk.inner.meta.get_or_insert_default(),
            AgenticLoopChunk::UserResource(chunk) => chunk.inner.meta.get_or_insert_default(),
            AgenticLoopChunk::UserResourceLink(chunk) => chunk.inner.meta.get_or_insert_default(),
            AgenticLoopChunk::Usage(chunk) => chunk.meta.get_or_insert_default(),
            AgenticLoopChunk::Notification(chunk) => chunk.meta.get_or_insert_default(),
        };
        attest(meta, image, who);
    }
}
