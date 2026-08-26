//! Tool messages.

use diverge_provider_sdk::endpoints::agentic_loop::run::server::response::AgenticLoopChunk;
use serde::{Deserialize, Serialize};

use super::super::RichContent;

/// A tool message containing the result of a tool call.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
)]
pub struct ToolMessage {
    /// The content of the tool response.
    pub content: RichContent,
    /// The ID of the tool call this message responds to.
    pub tool_call_id: String,
}

impl ToolMessage {
    /// A tool response chunk, as the message it is: the result's
    /// content blocks become parts, and the chunk's id names the call
    /// this answers. `is_error` and structured content have no
    /// message home and are dropped.
    ///
    /// Only `tool_response` chunks arrive here — the caller controls
    /// that — so every other kind is unreachable.
    pub fn new(chunk: AgenticLoopChunk) -> Self {
        let AgenticLoopChunk::ToolResponse(chunk) = chunk else {
            unreachable!("only tool response chunks are given here");
        };
        ToolMessage {
            content: RichContent::Parts(
                chunk.inner.content.into_iter().map(Into::into).collect(),
            ),
            tool_call_id: chunk.id,
        }
    }
}
