//! The `mcp_tool_call` item: a call to an MCP tool.

use serde::Deserialize;
use serde_json::Value;

/// A call to an MCP tool — through the proxy, since `diverge` is the
/// one server the harness configures. Started when the invocation is
/// dispatched (no result, no error, in progress), completed when the
/// server reports success or failure.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct McpToolCall {
    /// The server's name in `mcp_servers` — `diverge`, here.
    pub server: String,
    /// The tool's name, as the server lists it.
    pub tool: String,
    /// The arguments the model sent, as JSON. The source defaults it
    /// to `null` when absent.
    #[serde(default)]
    pub arguments: Value,
    /// The server's result, once there is one.
    pub result: Option<McpToolCallResult>,
    /// The failure, when the call failed.
    pub error: Option<McpToolCallError>,
    /// Where the call stands.
    pub status: McpToolCallStatus,
}

/// What an MCP tool returned: MCP's `CallToolResult`, its content
/// blocks kept as raw JSON — the source keeps them wire-shaped on
/// purpose, and so does this type; the converter turns them into
/// rmcp's own when it builds the `tool_response` chunk.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct McpToolCallResult {
    /// The content blocks, verbatim.
    pub content: Vec<Value>,
    /// MCP's `_meta`, when the server sent one.
    #[serde(rename = "_meta", default)]
    pub meta: Option<Value>,
    /// The structured content, when the server sent one.
    pub structured_content: Option<Value>,
}

/// Why an MCP tool call failed.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct McpToolCallError {
    /// The message.
    pub message: String,
}

/// The status of an MCP tool call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum McpToolCallStatus {
    /// Dispatched, unanswered.
    #[default]
    InProgress,
    /// Answered.
    Completed,
    /// Failed.
    Failed,
}
