//! Profile listing and usage response types.

use serde::{Deserialize, Serialize};

/// Response from listing profiles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListProfile {
    /// List of available profiles.
    pub data: Vec<ListProfileItem>,
}

/// A profile in a list response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListProfileItem {
    /// GitHub repository owner.
    pub owner: String,
    /// GitHub repository name.
    pub repository: String,
    /// Git commit SHA.
    pub commit: String,
}

/// Usage statistics for a profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageProfile {
    /// Total number of requests made with this profile.
    pub requests: u64,
    /// Total completion tokens used.
    pub completion_tokens: u64,
    /// Total prompt tokens used.
    pub prompt_tokens: u64,
    /// Total cost incurred.
    pub total_cost: rust_decimal::Decimal,
}
