//! Token usage.

use serde::{Deserialize, Serialize};

/// Token usage.
///
/// One type at both scales: a single turn reports its own, and the
/// loop reports the sum of them across turns, tool rounds and
/// fallbacks. The loop-level total appears once, on the terminal
/// chunk, because it is not final until the loop is.
///
/// Every field is additive, which is what lets one type serve both
/// scales — an aggregate is the sum of its parts and nothing more.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Usage {
    /// Tokens generated.
    pub completion_tokens: u64,
    /// Prompt tokens consumed.
    pub prompt_tokens: u64,
    /// The two above, summed.
    pub total_tokens: u64,
}

impl Usage {
    /// Whether anything at all was used.
    pub fn any_usage(&self) -> bool {
        self.completion_tokens > 0
            || self.prompt_tokens > 0
            || self.total_tokens > 0
    }

    /// Sum another usage into this one.
    pub fn push(&mut self, other: &Usage) {
        self.completion_tokens += other.completion_tokens;
        self.prompt_tokens += other.prompt_tokens;
        self.total_tokens += other.total_tokens;
    }
}
