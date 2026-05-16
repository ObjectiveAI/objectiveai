use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Response from the error endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "error.ErrorResponse")]
pub struct ErrorResponse {
    /// Whether the request completed successfully.
    pub ok: bool,
}
