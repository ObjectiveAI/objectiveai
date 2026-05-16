//! Request types for invention prompt endpoints.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Query parameters for the list prompts endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.inventions.prompts.ListPromptsRequest")]
pub struct ListPromptsRequest {
    /// Optional source filter for listing prompts.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub source: Option<ListPromptsSource>,
}

/// Source filter for listing prompts.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.inventions.prompts.ListPromptsSource")]
#[serde(rename_all = "snake_case")]
pub enum ListPromptsSource {
    All,
    Mock,
    Filesystem,
    Objectiveai,
}

/// Query parameters for getting a specific prompt.
pub type GetPromptRequest = crate::RemotePathCommitOptional;

impl ListPromptsSource {
    pub fn as_str(&self) -> &str {
        match self {
            ListPromptsSource::All => "all",
            ListPromptsSource::Mock => "mock",
            ListPromptsSource::Filesystem => "filesystem",
            ListPromptsSource::Objectiveai => "objectiveai",
        }
    }
}
