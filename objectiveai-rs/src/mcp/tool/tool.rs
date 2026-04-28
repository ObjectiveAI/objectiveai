//! MCP Tool definition.

use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A tool that an MCP server exposes for invocation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "mcp.tool.Tool")]
pub struct Tool {
    /// The programmatic name of the tool.
    pub name: String,
    /// A human-readable display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub title: Option<String>,
    /// A human-readable description of the tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub description: Option<String>,
    /// Icons for the tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub icons: Option<Vec<super::super::shared::Icon>>,
    /// JSON Schema defining the expected input parameters.
    /// Must have `type: "object"` at the root level.
    #[serde(rename = "inputSchema")]
    pub input_schema: ToolSchemaObject,
    /// JSON Schema defining the structure of the tool's output
    /// (returned in `structuredContent`).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[serde(rename = "outputSchema")]
    pub output_schema: Option<ToolSchemaObject>,
    /// Additional tool metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub annotations: Option<super::ToolAnnotations>,
    /// Execution-related properties.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub execution: Option<super::ToolExecution>,
    /// Extension metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub _meta: Option<IndexMap<String, serde_json::Value>>,
}

impl Tool {
    /// Returns a key identifying this tool, scoped to its connection.
    pub fn tool_key(&self, connection_tool_key: &str) -> String {
        format!("{connection_tool_key}-{}", self.name)
    }
}

/// The type of a JSON Schema used by MCP tools.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(rename = "mcp.tool.ToolSchemaType")]
pub enum ToolSchemaType {
    Object,
}

/// JSON Schema for tool input/output. Must have `type: "object"`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "mcp.tool.ToolSchemaObject")]
pub struct ToolSchemaObject {
    /// Always "object".
    pub r#type: ToolSchemaType,
    /// Property definitions.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub properties: Option<IndexMap<String, serde_json::Value>>,
    /// Required property names.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub required: Option<Vec<String>>,
    /// Additional schema fields.
    #[serde(flatten)]
    pub extra: IndexMap<String, serde_json::Value>,
}
