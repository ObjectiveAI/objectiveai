//! Response format configuration for agent completions.

use std::hash::{Hash, Hasher};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// The format of the model's response.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "agent.completions.request.ResponseFormat")]
pub enum ResponseFormat {
    /// Plain text response (default).
    #[schemars(title = "Text")]
    Text,
    /// Response must be valid JSON.
    #[schemars(title = "JsonObject")]
    JsonObject,
    /// Response must conform to a JSON schema.
    #[schemars(title = "JsonSchema")]
    JsonSchema {
        /// The JSON Schema definition.
        schema: IndexMap<String, serde_json::Value>,
    },
    /// Response must conform to a grammar.
    #[schemars(title = "Grammar")]
    Grammar { grammar: String },
    /// Response must be valid Python code.
    #[schemars(title = "Python")]
    Python,
    /// The final assistant message will contain this tool call
    #[schemars(title = "ToolCall")]
    ToolCall {
        /// The name of the tool.
        name: String,
        /// A description of the tool.
        description: String,
        /// The JSON Schema definition.
        schema: IndexMap<String, serde_json::Value>,
        /// Whether the tool MUST be called.
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        required: Option<bool>,
    },
}

impl ResponseFormat {
    pub fn tool_key(&self) -> String {
        let json = serde_json::to_string(self).unwrap_or_default();
        let mut h = std::collections::hash_map::DefaultHasher::new();
        json.hash(&mut h);
        format!("{:x}", h.finish())
    }
}

/// A map from agent ID to response format, allowing per-agent configuration.
pub type PerAgentResponseFormat = IndexMap<String, ResponseFormat>;

/// Either a single response format or a per-agent map.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "agent.completions.request.ResponseFormatParam")]
pub enum ResponseFormatParam {
    /// A single response format applied to all agents.
    #[schemars(title = "Single")]
    Single(ResponseFormat),
    /// Per-agent response formats, keyed by agent ID.
    #[schemars(title = "PerAgent")]
    PerAgent(PerAgentResponseFormat),
}
