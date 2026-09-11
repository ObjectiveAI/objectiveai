//! The `run.failed` event.

use serde::Deserialize;

/// The run died. Three producers, one shape: a structured failure
/// the agent returned, a provider-auth failure (its message leads
/// with a warning emoji), or a generic exception — in every case
/// [`error`](Self::error) is a plain redacted string, never a
/// structured object, never null.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RunFailed {
    /// The discriminator. Always `run.failed`.
    pub event: RunFailedEvent,
    /// The run.
    pub run_id: String,
    /// Seconds since the epoch, fractional.
    pub timestamp: f64,
    /// The failure, in the gateway's words.
    pub error: String,
}

/// [`RunFailed`]'s discriminator: the one value no other event
/// carries, which is what lets the union stay untagged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum RunFailedEvent {
    /// The only value.
    #[serde(rename = "run.failed")]
    RunFailed,
}
