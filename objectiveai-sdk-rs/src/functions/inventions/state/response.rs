//! Response types for function invention state retrieval.

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// Response from retrieving a function invention state.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.inventions.state.GetFunctionInventionStateResponse")]
pub struct GetFunctionInventionStateResponse {
    #[serde(flatten)]
    #[schemars(schema_with = "crate::flatten_schema::<crate::RemotePath>")]
    pub path: crate::RemotePath,
    #[serde(flatten)]
    #[schemars(schema_with = "crate::flatten_schema::<super::ParamsState>")]
    pub inner: super::ParamsState,
}
