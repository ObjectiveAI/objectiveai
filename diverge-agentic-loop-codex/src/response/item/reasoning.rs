//! The `reasoning` item: the agent's reasoning summary.

use serde::Deserialize;

/// A reasoning summary — what the agent's
/// [`reasoning_summary`](crate::agent::Agent::reasoning_summary)
/// allows the model to say of its thinking. Never started; whole at
/// `item.completed`. Raw reasoning is never on this wire.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Reasoning {
    /// The summary's text.
    pub text: String,
}
