//! Response types for function invention state retrieval.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Response from retrieving a function invention state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(
    rename = "functions.inventions.state.GetFunctionInventionStateResponse"
)]
pub struct GetFunctionInventionStateResponse {
    #[serde(flatten)]
    #[schemars(schema_with = "crate::flatten_schema::<crate::RemotePath>")]
    pub path: crate::RemotePath,
    #[serde(flatten)]
    #[schemars(schema_with = "crate::flatten_schema::<super::ParamsState>")]
    pub inner: super::ParamsState,
}
