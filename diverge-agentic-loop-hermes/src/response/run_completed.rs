//! The `run.completed` event.

use serde::Deserialize;

/// The run finished whole: the final response and the bill.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RunCompleted {
    /// The discriminator. Always `run.completed`.
    pub event: RunCompletedEvent,
    /// The run.
    pub run_id: String,
    /// Seconds since the epoch, fractional.
    pub timestamp: f64,
    /// The final response's text — the empty string when the agent
    /// returned nothing shaped like one; never null.
    pub output: String,
    /// The bill. See [`Usage`].
    pub usage: Usage,
    /// Steer text that arrived too late to enter the conversation,
    /// drained by the turn finalizer — ABSENT unless there was
    /// some.
    #[serde(default)]
    pub pending_steer: Option<String>,
}

/// The session's token accounting — exactly these three, nothing
/// else on this path (no cost, no reasoning split, no runtime).
///
/// `f64` deliberately: the gateway forwards the agent's counters
/// without coercion, and a count that reads is worth more than a
/// type that flatters it — in practice these are whole numbers.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Usage {
    /// Prompt-side tokens across the session.
    pub input_tokens: f64,
    /// Completion-side tokens across the session.
    pub output_tokens: f64,
    /// The two together, as the agent counted them.
    pub total_tokens: f64,
}

/// [`RunCompleted`]'s discriminator: the one value no other event
/// carries, which is what lets the union stay untagged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum RunCompletedEvent {
    /// The only value.
    #[serde(rename = "run.completed")]
    RunCompleted,
}
