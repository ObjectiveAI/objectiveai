//! The agentic loop chunk — the unit of a streaming response.

use serde::{Deserialize, Serialize};

use super::{
    AssistantAudioContentChunk, AssistantImageContentChunk,
    AssistantReasoningChunk, AssistantRefusalChunk,
    AssistantTextContentChunk, AssistantToolCallChunk, ContinuationChunk,
    NotificationChunk,
    ToolResponseChunk, UsageChunk,
};

/// One chunk of a streaming agentic loop.
///
/// Each chunk is ONE event, not a struct of mostly-absent optionals.
/// A consumer learns what happened by matching, rather than by
/// inspecting which fields happen to be set.
///
/// **Untagged, discriminated by payload.** serde adds no tag of its
/// own; instead every variant's payload carries a `type` field whose
/// value no other variant can produce. So the wire shape is the event
/// itself rather than a wrapper around one, and deserialization is
/// still unambiguous — the `type` constants do the work a tag would,
/// without a level of nesting.
///
/// That also means variant order here is not load-bearing. Untagged
/// deserialization takes the first variant that matches, and with
/// distinct `type` constants at most one ever can.
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
    /// Token usage so far. See [`UsageChunk`].
    Usage(UsageChunk),
    /// Something about the run itself. See [`NotificationChunk`].
    Notification(NotificationChunk),
    /// The resume token. See [`ContinuationChunk`].
    Continuation(ContinuationChunk),
}
