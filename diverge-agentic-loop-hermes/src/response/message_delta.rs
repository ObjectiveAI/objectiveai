//! The `message.delta` event.

use serde::Deserialize;

/// A piece of the assistant's text — token chunks while a response
/// streams, or one whole final response in a single delta. There
/// is no per-message framing around these: no started, no
/// completed; the deltas simply are the message.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct MessageDelta {
    /// The discriminator. Always `message.delta`.
    pub event: MessageDeltaEvent,
    /// The run.
    pub run_id: String,
    /// Seconds since the epoch, fractional.
    pub timestamp: f64,
    /// The text, never null (the producer filters `None`, its own
    /// end-of-stream marker, before the wire).
    pub delta: String,
}

/// [`MessageDelta`]'s discriminator: the one value no other event
/// carries, which is what lets the union stay untagged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum MessageDeltaEvent {
    /// The only value.
    #[serde(rename = "message.delta")]
    MessageDelta,
}
