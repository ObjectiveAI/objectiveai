//! Profile listing and usage response types.

use crate::functions;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// Response from listing profiles.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.profiles.ListProfileResponse")]
pub struct ListProfileResponse {
    /// List of available profiles.
    pub data: Vec<ListProfileItem>,
}

/// A profile in a list response.
pub type ListProfileItem = crate::RemotePath;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.profiles.GetProfileResponse")]
pub struct GetProfileResponse {
    #[serde(flatten)]
    #[schemars(schema_with = "crate::flatten_schema::<crate::RemotePath>")]
    pub path: crate::RemotePath,
    #[serde(flatten)]
    #[schemars(schema_with = "crate::flatten_schema::<functions::RemoteProfile>")]
    pub inner: functions::RemoteProfile,
}

/// Usage statistics for a profile.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.profiles.UsageProfileResponse")]
pub struct UsageProfileResponse {
    /// Total number of requests made with this profile.
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
