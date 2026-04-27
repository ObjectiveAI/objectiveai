//! Tool use content block.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// A tool call request from an assistant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUseContent {
    /// The name of the tool to invoke.
    pub name: String,
    /// Unique identifier for this tool call.
    pub id: String,
    /// Arguments to pass to the tool.
    pub input: IndexMap<String, serde_json::Value>,
    /// Extension metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<IndexMap<String, serde_json::Value>>,
}
