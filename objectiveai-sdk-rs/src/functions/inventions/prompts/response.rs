//! Invention prompt listing and usage response types.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Response from listing prompts.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.inventions.prompts.ListPromptResponse")]
pub struct ListPromptResponse {
    /// List of available prompts.
    pub data: Vec<ListPromptItem>,
}

/// A prompt in a list response.
pub type ListPromptItem = crate::RemotePath;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.inventions.prompts.GetPromptResponse")]
pub struct GetPromptResponse {
    #[serde(flatten)]
    #[schemars(schema_with = "crate::flatten_schema::<crate::RemotePath>")]
    pub path: crate::RemotePath,
    #[serde(flatten)]
    #[schemars(schema_with = "crate::flatten_schema::<super::RemotePrompt>")]
    pub inner: super::RemotePrompt,
}

/// Usage statistics for a prompt.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.inventions.prompts.UsagePromptResponse")]
pub struct UsagePromptResponse {
    /// Total number of requests made with this prompt.
    pub requests: u64,
    /// Total completion tokens used.
    pub completion_tokens: u64,
    /// Total prompt tokens used.
    pub prompt_tokens: u64,
    /// Total cost incurred.
    #[serde(deserialize_with = "crate::serde_util::decimal")]
    #[schemars(with = "f64")]
    pub total_cost: rust_decimal::Decimal,
}
