//! The `run.steered` event.

use serde::Deserialize;

/// A steer landed — pushed by the steer route when its POST
/// succeeds. A refused steer is that POST's own 409 and never
/// appears here, which is why [`accepted`](Self::accepted) is
/// always literally true on the wire.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RunSteered {
    /// The discriminator. Always `run.steered`.
    pub event: RunSteeredEvent,
    /// The run.
    pub run_id: String,
    /// Seconds since the epoch, fractional.
    pub timestamp: f64,
    /// Always true; read anyway, because a field the producer
    /// writes is a field this vocabulary reads.
    pub accepted: bool,
}

/// [`RunSteered`]'s discriminator: the one value no other event
/// carries, which is what lets the union stay untagged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum RunSteeredEvent {
    /// The only value.
    #[serde(rename = "run.steered")]
    RunSteered,
}
