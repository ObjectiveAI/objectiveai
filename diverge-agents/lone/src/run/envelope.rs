//! What the harness answers.

use serde::Deserialize;

/// The last line of the harness's stdout: the last expression's
/// value, and everything the harness printed.
///
/// One or the other is the turn — [`output`](super::output()) takes
/// `eval` when it is anything but `null`, and otherwise reads the
/// printed text as JSON — so a harness may return its chunks or print
/// them, and a harness that does neither has said nothing.
#[derive(Debug, Clone, Deserialize)]
pub struct Envelope {
    /// The last expression's value; `null` when there was no bare
    /// trailing expression, or it evaluated to `None`.
    pub eval: serde_json::Value,
    /// Everything the harness wrote to stdout, captured whole.
    pub stdout: String,
}
