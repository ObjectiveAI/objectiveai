//! A tool call that failed, as the tool's log keeps it.

use serde::{Deserialize, Serialize};

use crate::shared;

/// A [`Call`](super::Call) the tool did not answer: it could not be
/// started, or the exchange failed — in the words the daemon has.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct CallError {
    /// The call's `logs_index`.
    pub call: u64,
    /// The error, whole.
    pub error: shared::error::Error,
}
