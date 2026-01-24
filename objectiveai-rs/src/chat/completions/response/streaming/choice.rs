//! Choice type for streaming chat completion responses.

use crate::chat::completions::response;
use serde::{Deserialize, Serialize};

/// A choice in a streaming chat completion chunk.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Choice {
    /// The content delta for this choice.
    pub delta: super::Delta,
    /// The reason generation stopped, if complete.
    pub finish_reason: Option<response::FinishReason>,
    /// The index of this choice.
    pub index: u64,
    /// Log probabilities for tokens, if requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<response::Logprobs>,
}

impl Choice {
    /// Accumulates another choice into this one.
    pub fn push(
        &mut self,
        Choice {
            delta,
            finish_reason,
            logprobs,
            ..
        }: &Choice,
    ) {
        self.delta.push(delta);
        if self.finish_reason.is_none() {
            self.finish_reason = finish_reason.clone();
        }
        match (&mut self.logprobs, logprobs) {
            (Some(self_logprobs), Some(other_logprobs)) => {
                self_logprobs.push(other_logprobs);
            }
            (None, Some(other_logprobs)) => {
                self.logprobs = Some(other_logprobs.clone());
            }
            _ => {}
        }
    }
}
