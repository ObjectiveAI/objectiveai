//! The assistant tool call chunk.

use rmcp::model::RequestMetaObject;
use serde::{Deserialize, Serialize};

/// The model calling a tool.
///
/// A delta, like the text chunks around it. Providers stream tool
/// arguments in fragments, and this chunk carries them as they come:
/// consecutive tool call chunks bearing the same `id` continue one
/// call, their `arguments` concatenating in the order sent; a chunk
/// with a new `id` is a new call. The concatenation is the call's
/// arguments as JSON text, complete only when the last fragment has
/// arrived.
///
/// The fields are this chunk's own, spelled bare — they no longer
/// ride MCP's `CallToolRequestParams`, which is what made a delta
/// form possible: a fragment of a JSON object is not a JSON object,
/// but a fragment of a string is a string.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssistantToolCallChunk {
    /// The discriminator. See [`AgenticLoopChunk`](super::AgenticLoopChunk).
    pub r#type: AssistantToolCallChunkType,
    /// The tool call whose sub-agent produced this chunk; absent on
    /// the main thread. A nested sub-agent names its IMMEDIATE
    /// spawning call, so depth is a chain of ids a caller can follow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_tool_call_id: Option<String>,
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
    /// One fragment of the call's arguments: JSON text, complete
    /// only once every fragment with this `id` has been
    /// concatenated. Matches the tool's input schema when whole.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
}

/// [`AssistantToolCallChunk`]'s discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AssistantToolCallChunkType {
    #[serde(rename = "assistant_tool_call")]
    #[default]
    AssistantToolCall,
}

impl AssistantToolCallChunk {
    /// Merge the fragment that continued this call — same `id`, the
    /// caller checks — by appending its piece of the arguments. What
    /// else the fragment carried says nothing new about a call it is
    /// the continuation of, and is dropped.
    pub fn push(&mut self, other: Self) {
        if let Some(arguments) = other.arguments {
            match &mut self.arguments {
                Some(existing) => existing.push_str(&arguments),
                None => self.arguments = Some(arguments),
            }
        }
    }
}
