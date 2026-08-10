//! A tool call the model made.

use rmcp::model::CallToolRequestParams;
use serde::{Deserialize, Serialize};

/// One tool call, as it appears inside an
/// [`AssistantMessage`](super::AssistantMessage).
///
/// The same pair of fields as
/// [`AssistantToolCallChunk`](crate::agentic_loop::response::AssistantToolCallChunk)
/// minus its chunk discriminator: what the loop emitted is what the
/// caller sends back, so replaying a conversation is a copy rather
/// than a translation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    /// This call's id. A [`ToolMessage`](super::ToolMessage) answering
    /// it repeats this value — the only thing tying the two together.
    pub id: String,
    /// The call itself — tool name and arguments.
    #[serde(flatten)]
    pub inner: CallToolRequestParams,
}
