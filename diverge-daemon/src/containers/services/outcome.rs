//! What one start of a service's container came to.

/// The supervisor's last start: up, or why not, in a sentence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The run is up and live.
    Up,
    /// The start failed, in the start's words.
    Failed(String),
}
