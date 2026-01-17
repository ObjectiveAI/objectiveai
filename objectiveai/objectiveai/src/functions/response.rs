//! Function listing and usage response types.

use serde::{Deserialize, Serialize};

/// Response from listing functions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListFunction {
    /// List of available functions.
    pub data: Vec<ListFunctionItem>,
}

/// A function in a list response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListFunctionItem {
    /// GitHub repository owner.
    pub owner: String,
    /// GitHub repository name.
    pub repository: String,
    /// Git commit SHA.
    pub commit: String,
}

/// Usage statistics for a function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageFunction {
    /// Total number of requests made with this function.
    pub requests: u64,
    /// Total completion tokens used.
    pub completion_tokens: u64,
    /// Total prompt tokens used.
    pub prompt_tokens: u64,
    /// Total cost incurred.
    pub total_cost: rust_decimal::Decimal,
}
