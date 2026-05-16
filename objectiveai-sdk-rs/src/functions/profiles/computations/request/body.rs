use crate::{agent, functions};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// Parameters for creating a function profile computation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.profiles.computations.request.FunctionProfileComputationCreateParams")]
pub struct FunctionProfileComputationCreateParams {
    /// The function to compute a profile for (inline definition or remote path).
    pub function: functions::FullInlineFunctionOrRemoteCommitOptional,

    // --- Caching and retry options ---
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub retry_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub from_cache: Option<bool>,

    // --- Core configuration ---
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub max_retries: Option<u64>,
    pub n: u64,
    pub dataset: Vec<super::DatasetItem>,
    pub swarm: crate::swarm::InlineSwarmBaseOrRemoteCommitOptional,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub provider: Option<agent::completions::request::Provider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub stream: Option<bool>,

}
