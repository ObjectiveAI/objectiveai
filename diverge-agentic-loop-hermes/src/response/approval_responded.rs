//! The `approval.responded` event.

use serde::Deserialize;

/// A permission decision landed — pushed by the approval route
/// itself when its POST succeeds, so the stream's watchers learn
/// the run is moving again.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ApprovalResponded {
    /// The discriminator. Always `approval.responded`.
    pub event: ApprovalRespondedEvent,
    /// The run.
    pub run_id: String,
    /// Seconds since the epoch, fractional.
    pub timestamp: f64,
    /// The decision, normalized by the gateway to one of `once`,
    /// `session`, `always`, `deny`.
    pub choice: String,
    /// How many pending entries the decision resolved — at least
    /// one, since resolving none answers 409 and emits nothing.
    pub resolved: u64,
}

/// [`ApprovalResponded`]'s discriminator: the one value no other
/// event carries, which is what lets the union stay untagged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum ApprovalRespondedEvent {
    /// The only value.
    #[serde(rename = "approval.responded")]
    ApprovalResponded,
}
