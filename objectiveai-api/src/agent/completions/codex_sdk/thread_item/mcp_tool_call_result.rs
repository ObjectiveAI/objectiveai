use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct McpToolCallResult {
    /// MCP content blocks. Modeled as raw JSON values so the spec can evolve
    /// (the MCP content-block schema is itself a discriminated union we
    /// don't enumerate here).
    pub content: Vec<serde_json::Value>,
    /// Whatever the MCP server returned as its structured payload — by
    /// definition arbitrary JSON.
    pub structured_content: serde_json::Value,
}
