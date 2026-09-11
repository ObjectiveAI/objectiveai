//! The `run.cancelled` event.

use serde::Deserialize;

/// The run was stopped — the trio and nothing else, no reason
/// field, from any of its three producers (a stop that beat the
/// agent's start, a stop noticed after the executor returned, or
/// the task's own cancellation). NOTE the stop route itself emits
/// nothing; this arrives only when the run actually notices.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RunCancelled {
    /// The discriminator. Always `run.cancelled`.
    pub event: RunCancelledEvent,
    /// The run.
    pub run_id: String,
    /// Seconds since the epoch, fractional.
    pub timestamp: f64,
}

/// [`RunCancelled`]'s discriminator: the one value no other event
/// carries, which is what lets the union stay untagged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum RunCancelledEvent {
    /// The only value.
    #[serde(rename = "run.cancelled")]
    RunCancelled,
}
