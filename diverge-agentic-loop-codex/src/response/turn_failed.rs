//! The `turn.failed` event: a turn ending in failure.

use serde::Deserialize;

use super::ThreadError;

/// A `type: "turn.failed"` event, written when the turn's status is
/// failed. Its error is the turn's own, or — when the turn carried
/// none — the last critical [`Error`](super::Error) event, or the
/// bare "turn failed". The process shuts down after it, and no final
/// message is kept.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TurnFailed {
    /// Always `turn.failed`.
    pub r#type: TurnFailedType,
    /// Why.
    pub error: ThreadError,
}

/// The `turn.failed` literal.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize)]
pub enum TurnFailedType {
    /// The only value.
    #[default]
    #[serde(rename = "turn.failed")]
    TurnFailed,
}
