//! What the harness answers.

use serde::Deserialize;

/// The last line of the harness's stdout: the last expression's
/// value, and everything the script printed.
///
/// One or the other is the turn — [`output`](super::output()) takes
/// `eval` when it is anything but `null`, and otherwise reads the
/// printed text as JSON — so a script may return its chunks or print
/// them, and a script that does neither has said nothing.
#[derive(Debug, Clone, Deserialize)]
pub struct Envelope {
    /// The last expression's value; `null` when there was no bare
    /// trailing expression, or it evaluated to `None`.
    pub eval: serde_json::Value,
    /// Everything the script wrote to stdout, captured whole.
    pub stdout: String,
}
