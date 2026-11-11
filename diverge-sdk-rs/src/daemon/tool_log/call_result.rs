//! A tool call's result, as the tool's log keeps it.

use rmcp::model::CallToolResult;
use serde::{Deserialize, Serialize};

/// The tool's answer to a [`Call`](super::Call), whole, as the tool
/// gave it — an answer the tool marked an error among them, since that
/// is the tool's word and not a failure to answer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct CallResult {
    /// The call's `logs_index`.
    pub call: u64,
    /// The result.
    pub result: CallToolResult,
}
