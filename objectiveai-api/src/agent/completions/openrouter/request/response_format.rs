//! Response format construction for chat completions.

use serde::{Deserialize, Serialize};

/// The format of the model's response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

impl ResponseFormat {
    /// Converts a non-ToolCall objectiveai ResponseFormat to the OpenRouter ResponseFormat.
    ///
    /// # Panics
    ///
    /// Panics if called with the `ToolCall` variant — that variant must be
    /// extracted into a tool before reaching this method.
    pub fn new(rf: &objectiveai_sdk::agent::completions::request::ResponseFormat) -> Self {
        match rf {
            objectiveai_sdk::agent::completions::request::ResponseFormat::Text => Self::Text,
            objectiveai_sdk::agent::completions::request::ResponseFormat::JsonObject => {
                Self::JsonObject
            }
            objectiveai_sdk::agent::completions::request::ResponseFormat::JsonSchema {
                schema,
            } => Self::JsonSchema {
                json_schema: JsonSchema {
                    name: schema
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("response")
                        .to_string(),
                    description: schema
                        .get("description")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    schema: Some(serde_json::Value::Object(
                        schema
                            .iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect(),
                    )),
                    strict: None,
                },
            },
            objectiveai_sdk::agent::completions::request::ResponseFormat::Grammar {
                grammar,
            } => Self::Grammar {
                grammar: grammar.clone(),
            },
            objectiveai_sdk::agent::completions::request::ResponseFormat::Python => Self::Python,
            objectiveai_sdk::agent::completions::request::ResponseFormat::ToolCall { .. } => {
                unreachable!(
                    "ToolCall variant should be handled before calling ResponseFormat::new"
                )
            }
        }
    }
}

/// A JSON schema for structured output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
