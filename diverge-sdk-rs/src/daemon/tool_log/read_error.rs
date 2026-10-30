//! A resource read that failed, as the tool's log keeps it.

use serde::{Deserialize, Serialize};

use crate::shared;

/// A [`Read`](super::Read) the tool did not answer, or answered with
/// an error — in the words the daemon has. An agent's read is asked of
/// each served tool in turn until one answers; each tool asked keeps
/// its own read, and each that did not hold the resource its own error.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct ReadError {
    /// The read's `logs_index`.
    pub read: u64,
    /// The error, whole.
    pub error: shared::error::Error,
}
