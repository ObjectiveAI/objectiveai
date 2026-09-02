//! The tool response chunk.

use rmcp::model::CallToolResult;
use serde::{Deserialize, Serialize};

/// The result of one tool call.
///
/// Arrives whole, unlike the assistant chunks: a tool either returned
/// or it did not, so there is nothing to stream in pieces.
///
/// The result is MCP's own [`CallToolResult`], flattened, so what an
/// MCP server returned passes through verbatim — content blocks,
/// structured content, `isError` and `_meta` included — rather than
/// being re-encoded into a shape of ours that would lose some of it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolResponseChunk {
    /// The discriminator. See [`ContinuationChunk`](super::ContinuationChunk).
    pub r#type: ToolResponseChunkType,
    /// The tool call whose sub-agent produced this chunk; absent on
    /// the main thread. A nested sub-agent names its IMMEDIATE
    /// spawning call, so depth is a chain of ids a caller can follow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_tool_call_id: Option<String>,
    /// The call this answers.
    ///
    /// Ours, not MCP's: a [`CallToolResult`] carries no id at all,
    /// because in MCP it is the payload of a JSON-RPC response and the
    /// request id does the correlating from the envelope. A stream has
    /// no envelope, and results may arrive in a different order than
    /// the calls were made, so the link has to be here.
    pub id: String,
    /// The result itself.
    #[serde(flatten)]
    pub inner: CallToolResult,
}

/// [`ToolResponseChunk`]'s discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ToolResponseChunkType {
    #[serde(rename = "tool_response")]
    #[default]
    ToolResponse,
}
