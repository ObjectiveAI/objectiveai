//! The assistant tool call chunk.

use rmcp::model::CallToolRequestParams;
use serde::{Deserialize, Serialize};

/// The model calling a tool.
///
/// Whole, not a delta. Providers stream tool arguments in fragments,
/// but a fragment of a JSON object is not a JSON object — the same
/// reason the image and audio chunks arrive intact. A provider
/// assembles the arguments and emits one of these when there is
/// something a caller can act on.
///
/// The payload is MCP's [`CallToolRequestParams`], flattened — the
/// exact counterpart of the [`CallToolResult`](rmcp::model::CallToolResult)
/// that [`ToolResponseChunk`](super::ToolResponseChunk) carries. A
/// call and its result are described by the same pair of types MCP
/// uses for them, so neither direction needs translating before it
/// reaches a server.
///
/// `arguments` is a structured `JsonObject`, not a JSON string. There
/// is no encoding step, and therefore no way for the arguments to be
/// syntactically invalid by the time a caller reads them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssistantToolCallChunk {
    /// The discriminator. See [`ContinuationChunk`](super::ContinuationChunk).
    pub r#type: AssistantToolCallChunkType,
    /// This call's id, which its
    /// [`ToolResponseChunk`](super::ToolResponseChunk) echoes back.
    ///
    /// Ours, not MCP's: in MCP the JSON-RPC envelope correlates a
    /// request with its response, and a stream has no envelope.
    pub id: String,
    /// The call itself — tool name and arguments.
    #[serde(flatten)]
    pub inner: CallToolRequestParams,
}

/// [`AssistantToolCallChunk`]'s discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AssistantToolCallChunkType {
    #[serde(rename = "assistant_tool_call")]
    #[default]
    AssistantToolCall,
}
