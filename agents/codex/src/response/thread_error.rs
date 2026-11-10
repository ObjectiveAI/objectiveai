//! An error, as a payload.

use serde::Deserialize;

/// The source's `ThreadErrorEvent` where it rides INSIDE another
/// event — `turn.failed`'s `error` — with no `type` of its own. At
/// the top of a line the same payload is the [`Error`](super::Error)
/// event, marked.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ThreadError {
    /// The message, with the core's additional details folded in
    /// as ` (details)` when it had any.
    pub message: String,
}
