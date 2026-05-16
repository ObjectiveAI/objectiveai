use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Request to trigger an error response for testing purposes.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "error.ErrorCreateParams")]
pub struct ErrorCreateParams {
    /// Random seed for deterministic error generation.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub seed: Option<i64>,
    /// Whether to stream the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub stream: Option<bool>,
}
