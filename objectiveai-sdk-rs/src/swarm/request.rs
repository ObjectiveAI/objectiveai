//! Request types for swarm listing endpoints.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Query parameters for the list swarms endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "swarm.ListSwarmsRequest")]
pub struct ListSwarmsRequest {
    /// Optional source filter for listing swarms.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub source: Option<ListSwarmsSource>,
}

/// Source filter for listing swarms.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "swarm.ListSwarmsSource")]
#[serde(rename_all = "snake_case")]
pub enum ListSwarmsSource {
    All,
    Mock,
    Filesystem,
    Objectiveai,
}

impl ListSwarmsSource {
    pub fn as_str(&self) -> &str {
        match self {
            ListSwarmsSource::All => "all",
            ListSwarmsSource::Mock => "mock",
            ListSwarmsSource::Filesystem => "filesystem",
            ListSwarmsSource::Objectiveai => "objectiveai",
        }
    }
}

/// Request parameters for getting a specific swarm.
pub type GetSwarmRequest = crate::RemotePathCommitOptional;
