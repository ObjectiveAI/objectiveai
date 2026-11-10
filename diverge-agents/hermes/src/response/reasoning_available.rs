//! The `reasoning.available` event.

use serde::Deserialize;

/// A slice of the model's reasoning — truncated to 500 chars at
/// the SOURCE, before the gateway ever sees it, so this is a
/// glimpse and never the whole.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ReasoningAvailable {
    /// The discriminator. Always `reasoning.available`.
    pub event: ReasoningAvailableEvent,
    /// The run.
    pub run_id: String,
    /// Seconds since the epoch, fractional.
    pub timestamp: f64,
    /// The reasoning text, never null (the producer coalesces to
    /// the empty string).
    pub text: String,
}

/// [`ReasoningAvailable`]'s discriminator: the one value no other
/// event carries, which is what lets the union stay untagged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum ReasoningAvailableEvent {
    /// The only value.
    #[serde(rename = "reasoning.available")]
    ReasoningAvailable,
}
