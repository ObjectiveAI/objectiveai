//! The `turn.completed` event: a turn ending well.

use serde::Deserialize;

use super::Usage;

/// A `type: "turn.completed"` event, written right after the
/// assistant's response when the turn's status is completed. Before
/// it, a running todo list is completed and every started item not
/// yet completed is reconciled to completed. The process shuts down
/// after it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TurnCompleted {
    /// Always `turn.completed`.
    pub r#type: TurnCompletedType,
    /// The thread's usage so far — CUMULATIVE, not this turn's. See
    /// [`Usage`].
    pub usage: Usage,
}

/// The `turn.completed` literal.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize)]
pub enum TurnCompletedType {
    /// The only value.
    #[default]
    #[serde(rename = "turn.completed")]
    TurnCompleted,
}
