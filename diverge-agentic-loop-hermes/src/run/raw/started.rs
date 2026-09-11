//! What `POST /v1/runs` answers.

use serde::Deserialize;

/// The `202`: the run's id, and its status — `started`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Started {
    /// The run, for the events route.
    pub run_id: String,
    /// `started`, on the wire as written; kept, not judged.
    pub status: String,
}
