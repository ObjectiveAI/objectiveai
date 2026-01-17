//! Choice type for unary chat completions.

use crate::chat::completions::response;
use serde::{Deserialize, Serialize};

/// A generated choice in a chat completion response.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Choice {
    /// The generated message.
    pub message: super::Message,
    /// Why the model stopped generating.
    pub finish_reason: response::FinishReason,
    /// The index of this choice.
    pub index: u64,
    /// Log probabilities for the generated tokens.
    pub logprobs: Option<response::Logprobs>,
}

impl From<response::streaming::Choice> for Choice {
    fn from(
        response::streaming::Choice {
            delta,
            finish_reason,
            index,
            logprobs,
        }: response::streaming::Choice,
    ) -> Self {
        Self {
            message: super::Message::from(delta),
            finish_reason: finish_reason.unwrap_or_default(),
            index,
            logprobs,
        }
    }
}
