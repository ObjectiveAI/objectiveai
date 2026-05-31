//! Function listing and usage response types.

use crate::functions;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// Response from listing functions.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.ListFunctionResponse")]
pub struct ListFunctionResponse {
    /// List of available functions.
    pub data: Vec<ListFunctionItem>,
}

/// A function in a list response.
pub type ListFunctionItem = crate::RemotePath;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.GetFunctionResponse")]
pub struct GetFunctionResponse {
    #[serde(flatten)]
    #[schemars(schema_with = "crate::flatten_schema::<crate::RemotePath>")]
    pub path: crate::RemotePath,
    #[serde(flatten)]
    #[schemars(schema_with = "crate::flatten_schema::<functions::FullRemoteFunction>")]
    pub inner: functions::FullRemoteFunction,
}

/// Usage statistics for a function.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.UsageFunctionResponse")]
pub struct UsageFunctionResponse {
    /// Total number of requests made with this function.
    pub requests: u64,
    /// Total completion tokens used.
    pub completion_tokens: u64,
    /// Total prompt tokens used.
    pub prompt_tokens: u64,
    /// Total cost incurred.
    #[serde(deserialize_with = "crate::serde_util::decimal")]
    #[schemars(with = "f64")]
    pub total_cost: rust_decimal::Decimal,
}

/// Response from listing function-profile pairs.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.ListFunctionProfilePairResponse")]
pub struct ListFunctionProfilePairResponse {
    /// List of available function-profile pairs.
    pub data: Vec<ListFunctionProfilePairItem>,
}

/// A function-profile pair in a list response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.ListFunctionProfilePairItem")]
pub struct ListFunctionProfilePairItem {
    /// The function.
    pub function: ListFunctionItem,
    /// The profile.
    pub profile: functions::profiles::response::ListProfileItem,
}

/// Response from getting a function-profile pair.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.GetFunctionProfilePairResponse")]
pub struct GetFunctionProfilePairResponse {
    /// The function.
    pub function: GetFunctionResponse,
    /// The profile.
    pub profile: functions::profiles::response::GetProfileResponse,
}

/// Usage statistics for a function-profile pair.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.UsageFunctionProfilePairResponse")]
pub struct UsageFunctionProfilePairResponse {
    /// Total number of requests made with this function-profile pair.
    pub requests: u64,
    /// Total completion tokens used.
    pub completion_tokens: u64,
    /// Total prompt tokens used.
    pub prompt_tokens: u64,
    /// Total cost incurred.
    #[serde(deserialize_with = "crate::serde_util::decimal")]
    #[schemars(with = "f64")]
    pub total_cost: rust_decimal::Decimal,
}
