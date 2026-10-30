//! A resource read's result, as the tool's log keeps it.

use rmcp::model::ReadResourceResult;
use serde::{Deserialize, Serialize};

/// The tool's answer to a [`Read`](super::Read), whole.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct ReadResult {
    /// The read's `logs_index`.
    pub read: u64,
    /// The result.
    pub result: ReadResourceResult,
}
