//! The `reasoning` item: the agent's reasoning summary.

use serde::Deserialize;

/// A reasoning summary — what the agent's
/// [`reasoning_summary`](crate::agent::Agent::reasoning_summary)
/// allows the model to say of its thinking. Never started; whole at
/// `item.completed`. Raw reasoning is never on this wire. A summary
/// that is blank after trimming produces NO item at all, so a turn
/// may carry none however the setting reads.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Reasoning {
    /// The summary's lines, joined with `\n`.
    pub text: String,
}
