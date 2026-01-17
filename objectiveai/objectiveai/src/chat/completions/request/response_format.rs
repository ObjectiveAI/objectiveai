//! Response format configuration for chat completions.

use serde::{Deserialize, Serialize};

/// The format of the model's response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseFormat {
    /// Plain text response (default).
    Text,
    /// Response must be valid JSON.
    JsonObject,
    /// Response must conform to a JSON schema.
    JsonSchema { json_schema: JsonSchema },
    /// Response must conform to a grammar.
    Grammar { grammar: String },
    /// Response must be valid Python code.
    Python,
}

/// A JSON schema for structured output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchema {
    /// The name of the schema.
    pub name: String,
    /// A description of the schema's purpose.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The JSON Schema definition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<serde_json::Value>,
    /// Whether to enforce strict schema validation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}
