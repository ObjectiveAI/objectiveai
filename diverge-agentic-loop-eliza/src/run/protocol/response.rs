//! What the entry writes to this program.

use rmcp::model::CallToolResult;
use serde::Deserialize;
use serde_json::Value;

/// One line from the entry's stdout.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    /// A failure before `ready`; the entry exits right behind it.
    Fatal { error: String },
    /// The runtime is initialized and the room exists.
    Ready,
    /// A text delta of the turn's reply.
    Text { delta: String },
    /// The model calling a tool: the id its result echoes, the tool's
    /// name as the caller knows it, the arguments as JSON text.
    ToolCall {
        id: String,
        name: String,
        arguments: String,
    },
    /// A tool's result, MCP's own shape — verbatim for the caller's
    /// tools, rendered for Eliza's built-in actions.
    ToolResult { id: String, result: CallToolResult },
    /// One model call's token counts, as the provider reported them.
    Usage {
        prompt: u64,
        completion: u64,
        total: u64,
    },
    /// Something about the run worth saying, in the entry's words.
    Notification { message: Value },
    /// The turn's end: the authoritative final text, whether any
    /// delta was streamed, and the turn's terminal failure if any.
    Done {
        text: Option<String>,
        streamed: bool,
        failure: Option<Value>,
    },
    /// The answer to a `read`: the value, or nothing under the key.
    #[serde(rename = "value")]
    Answer { key: String, value: Option<String> },
    /// The runtime is stopped and the database closed; the entry
    /// exits right behind it.
    Stopped,
}
