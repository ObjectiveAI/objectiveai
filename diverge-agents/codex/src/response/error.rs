//! The `error` event: a critical error.

use serde::Deserialize;

/// A `type: "error"` event — the core reporting a critical error,
/// or the JSONL writer failing to serialize an event.
///
/// Not by itself the turn's end: the processor's status stays
/// running after it, and the turn ends with `turn.failed`, which
/// carries this message when the turn has none of its own. So a
/// reader holds it, and judges by what follows.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Error {
    /// Always `error`.
    pub r#type: ErrorType,
    /// The message, with the core's additional details folded in
    /// as ` (details)` when it had any.
    pub message: String,
}

/// The `error` literal.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorType {
    /// The only value.
    #[default]
    Error,
}
