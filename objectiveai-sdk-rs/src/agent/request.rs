//! Request types for agent listing endpoints.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Query parameters for the list agents endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.ListAgentsRequest")]
pub struct ListAgentsRequest {
    /// Optional source filter for listing agents.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub source: Option<ListAgentsSource>,
}

/// Source filter for listing agents.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.ListAgentsSource")]
#[serde(rename_all = "snake_case")]
pub enum ListAgentsSource {
    All,
    Mock,
    Filesystem,
    Objectiveai,
}

impl ListAgentsSource {
    pub fn as_str(&self) -> &str {
        match self {
            ListAgentsSource::All => "all",
            ListAgentsSource::Mock => "mock",
            ListAgentsSource::Filesystem => "filesystem",
            ListAgentsSource::Objectiveai => "objectiveai",
        }
    }
}

/// Request parameters for getting a specific agent.
pub type GetAgentRequest = crate::RemotePathCommitOptional;
