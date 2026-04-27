//! Types for tools/call requests and responses.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Metadata for a long-running task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetadata {
    /// Time-to-live for the task, in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<f64>,
}

/// Parameters for a `tools/call` request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolRequestParams {
    /// The name of the tool to call.
    pub name: String,
    /// Arguments to pass to the tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<IndexMap<String, serde_json::Value>>,
    /// Extension metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<IndexMap<String, serde_json::Value>>,
    /// If specified, the caller is requesting task-augmented execution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<TaskMetadata>,
}

/// The server's response to a `tools/call` request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolResult {
    /// Content blocks representing the result of the tool call.
    #[serde(default)]
    pub content: Vec<super::ContentBlock>,
    /// Structured tool output matching the tool's `outputSchema`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "structuredContent")]
    pub structured_content: Option<IndexMap<String, serde_json::Value>>,
    /// Whether the tool call ended in an error.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "isError")]
    pub is_error: Option<bool>,
    /// Extension metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<IndexMap<String, serde_json::Value>>,
}
