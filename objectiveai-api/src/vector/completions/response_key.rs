//! Response key schema generation for structured LLM voting.
//!
//! Provides JSON schema and tool definitions that constrain LLM output to
//! select one of the available response keys.

/// Parsed response key from LLM structured output.
#[derive(Debug, serde::Deserialize)]
pub struct ResponseKey {
    /// Optional synthetic reasoning from the LLM.
    pub _think: Option<String>,
    /// The selected response key.
    #[allow(dead_code)]
    response_key: String,
}

impl ResponseKey {
    /// Creates a JSON schema for response key selection.
    fn schema(
        vector_response_keys: Vec<String>,
        think: bool,
    ) -> serde_json::Map<String, serde_json::Value> {
        let mut map = serde_json::Map::with_capacity(4);
        map.insert(
            "type".to_string(),
            serde_json::Value::String("object".to_string()),
        );
        map.insert(
            "properties".to_string(),
            serde_json::Value::Object({
                let mut properties = serde_json::Map::with_capacity(if think { 2 } else { 1 });
                if think {
                    properties.insert(
                        "_think".to_string(),
                        serde_json::json!({
                            "type": "string",
                            "description": "The assistant's internal reasoning.",
                        }),
                    );
                }
                properties.insert(
                    "response_key".to_string(),
                    serde_json::json!({
                        "type": "string",
                        "enum": vector_response_keys
                    }),
                );
                properties
            }),
        );
        map.insert(
            "required".to_string(),
            serde_json::Value::Array({
                let mut required = Vec::with_capacity(if think { 2 } else { 1 });
                if think {
                    required.push(serde_json::Value::String("_think".to_string()));
                }
                required.push(serde_json::Value::String("response_key".to_string()));
                required
            }),
        );
        map.insert(
            "additionalProperties".to_string(),
            serde_json::Value::Bool(false),
        );
        map
    }

    /// Creates a response format for JSON schema output mode.
    pub fn response_format(
        vector_response_keys: Vec<String>,
        think: bool,
    ) -> objectiveai_sdk::agent::completions::request::ResponseFormat {
        let schema: indexmap::IndexMap<String, serde_json::Value> =
            Self::schema(vector_response_keys, think).into_iter().collect();
        objectiveai_sdk::agent::completions::request::ResponseFormat::JsonSchema {
            schema,
        }
    }

    /// Creates a tool definition for tool call output mode.
    pub fn tool(
        vector_response_keys: Vec<String>,
        think: bool,
    ) -> objectiveai_sdk::agent::completions::request::ResponseFormat {
        let schema: indexmap::IndexMap<String, serde_json::Value> =
            Self::schema(vector_response_keys, think).into_iter().collect();
        objectiveai_sdk::agent::completions::request::ResponseFormat::ToolCall {
            name: "response_key".to_string(),
            description: "Select the response key.".to_string(),
            schema,
            required: None,
        }
    }

    /// Creates a tool call response format with required set.
    pub fn tool_required(
        vector_response_keys: Vec<String>,
        think: bool,
    ) -> objectiveai_sdk::agent::completions::request::ResponseFormat {
        let schema: indexmap::IndexMap<String, serde_json::Value> =
            Self::schema(vector_response_keys, think).into_iter().collect();
        objectiveai_sdk::agent::completions::request::ResponseFormat::ToolCall {
            name: "response_key".to_string(),
            description: "Select the response key.".to_string(),
            schema,
            required: Some(true),
        }
    }
}
