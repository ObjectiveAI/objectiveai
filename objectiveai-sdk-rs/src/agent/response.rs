//! Response types for Agent API endpoints.

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// Response containing a list of Agents.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.ListAgentResponse")]
pub struct ListAgentResponse {
    /// The list of Agent summaries.
    pub data: Vec<ListAgentItem>,
}

/// Summary information for a listed Agent.
pub type ListAgentItem = crate::RemotePath;

/// Response containing a single Agent with creation timestamp.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.GetAgentResponse")]
pub struct GetAgentResponse {
    #[serde(flatten)]
    #[schemars(schema_with = "crate::flatten_schema::<crate::RemotePath>")]
    pub path: crate::RemotePath,
    /// The Agent definition.
    #[serde(flatten)]
    #[schemars(schema_with = "crate::flatten_schema::<super::RemoteAgentWithFallbacks>")]
    pub inner: super::RemoteAgentWithFallbacks,
}

/// Usage statistics for an Agent.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.UsageAgentResponse")]
pub struct UsageAgentResponse {
    /// Total number of requests made with this Agent.
    pub requests: u64,
    /// Total completion tokens generated.
    pub completion_tokens: u64,
    /// Total prompt tokens processed.
    pub prompt_tokens: u64,
    /// Total cost incurred.
    #[serde(deserialize_with = "crate::serde_util::decimal")]
    #[schemars(with = "f64")]
    pub total_cost: rust_decimal::Decimal,
}
