//! Tool result content block.

use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The result of a tool invocation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "mcp.tool.ToolResultContent")]
pub struct ToolResultContent {
    /// The unique identifier for the corresponding tool call.
    #[serde(rename = "toolUseId")]
    pub tool_use_id: String,
    /// Content blocks from the tool result.
    #[serde(default)]
    pub content: Vec<super::ContentBlock>,
    /// Structured content from the tool result.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[serde(rename = "structuredContent")]
    pub structured_content: Option<IndexMap<String, serde_json::Value>>,
    /// Whether this result represents an error.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[serde(rename = "isError")]
    pub is_error: Option<bool>,
    /// Extension metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub _meta: Option<IndexMap<String, serde_json::Value>>,
}
