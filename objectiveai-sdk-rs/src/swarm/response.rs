//! Response types for Swarm API endpoints.

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// Response containing a list of Swarms.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "swarm.ListSwarmResponse")]
pub struct ListSwarmResponse {
    /// The list of Swarm summaries.
    pub data: Vec<ListSwarmItem>,
}

/// Summary information for a listed Swarm.
pub type ListSwarmItem = crate::RemotePath;

/// Response containing a single Swarm.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "swarm.GetSwarmResponse")]
pub struct GetSwarmResponse {
    #[serde(flatten)]
    #[schemars(schema_with = "crate::flatten_schema::<crate::RemotePath>")]
    pub path: crate::RemotePath,
    /// The Swarm definition.
    #[serde(flatten)]
    #[schemars(schema_with = "crate::flatten_schema::<super::RemoteSwarm>")]
    pub inner: super::RemoteSwarm,
}

/// Usage statistics for a Swarm.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "swarm.UsageSwarmResponse")]
pub struct UsageSwarmResponse {
    /// Total number of requests made with this Swarm.
    pub requests: u64,
    /// Total completion tokens generated across all agents.
    pub completion_tokens: u64,
    /// Total prompt tokens processed across all agents.
    pub prompt_tokens: u64,
    /// Total cost incurred.
    #[serde(deserialize_with = "crate::serde_util::decimal")]
    #[schemars(with = "f64")]
    pub total_cost: rust_decimal::Decimal,
}
