//! Request types for profile listing endpoints.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Query parameters for the list profiles endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.profiles.ListProfilesRequest")]
pub struct ListProfilesRequest {
    /// Optional source filter for listing profiles.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub source: Option<ListProfilesSource>,
}

/// Source filter for listing profiles.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.profiles.ListProfilesSource")]
#[serde(rename_all = "snake_case")]
pub enum ListProfilesSource {
    All,
    Mock,
    Filesystem,
    Objectiveai,
}

/// Query parameters for getting a specific profile.
pub type GetProfileRequest = crate::RemotePathCommitOptional;

impl ListProfilesSource {
    pub fn as_str(&self) -> &str {
        match self {
            ListProfilesSource::All => "all",
            ListProfilesSource::Mock => "mock",
            ListProfilesSource::Filesystem => "filesystem",
            ListProfilesSource::Objectiveai => "objectiveai",
        }
    }
}
