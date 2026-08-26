//! The assistant tool call chunk.

use rmcp::model::{InputResponses, JsonObject, RequestMetaObject};
use serde::{Deserialize, Serialize};

/// The model calling a tool.
///
/// Whole, not a delta. Providers stream tool arguments in fragments,
/// but a fragment of a JSON object is not a JSON object — the same
/// reason the image and audio chunks arrive intact. A provider
/// assembles the arguments and emits one of these when there is
/// something a caller can act on.
///
/// The fields are this chunk's own, spelled bare. They used to arrive
/// as MCP's `CallToolRequestParams`, flattened; the wire shape is
/// unchanged — the same members in the same places — but the type no
/// longer rides MCP's, so what a tool call chunk carries is this
/// crate's to evolve.
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
    /// Protocol-level metadata for the call.
    #[serde(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<RequestMetaObject>,
    /// The name of the tool to call.
    pub name: String,
    /// Arguments to pass to the tool, matching the tool's input
    /// schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<JsonObject>,
    /// Client responses to server-initiated input requests from a
    /// previous incomplete result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_responses: Option<InputResponses>,
    /// Opaque request state echoed back from a previous incomplete
    /// result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_state: Option<String>,
}

/// [`AssistantToolCallChunk`]'s discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AssistantToolCallChunkType {
    #[serde(rename = "assistant_tool_call")]
    #[default]
    AssistantToolCall,
}
