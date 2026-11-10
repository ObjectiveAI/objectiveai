//! The `turn.started` event: a turn beginning.

use serde::Deserialize;

/// A `type: "turn.started"` event, written when the prompt is sent
/// to the model. A turn encompasses every event until its
/// `turn.completed` or `turn.failed`, and one process is one turn.
/// Carries nothing else.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TurnStarted {
    /// Always `turn.started`.
    pub r#type: TurnStartedType,
}

/// The `turn.started` literal.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize)]
pub enum TurnStartedType {
    /// The only value.
    #[default]
    #[serde(rename = "turn.started")]
    TurnStarted,
}
