//! Log probability information for generated tokens.

use serde::{Deserialize, Serialize};

/// Log probabilities for generated tokens.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Logprobs {
    /// Log probabilities for content tokens.
    pub content: Option<Vec<Logprob>>,
    /// Log probabilities for refusal tokens.
    pub refusal: Option<Vec<Logprob>>,
}

impl Logprobs {
    /// Appends log probabilities from another instance.
    pub fn push(&mut self, other: &Logprobs) {
        match (&mut self.content, &other.content) {
            (Some(self_content), Some(other_content)) => {
                self_content.extend(other_content.clone());
            }
            (None, Some(other_content)) => {
                self.content = Some(other_content.clone());
            }
            _ => {}
        }
        match (&mut self.refusal, &other.refusal) {
            (Some(self_refusal), Some(other_refusal)) => {
                self_refusal.extend(other_refusal.clone());
            }
            (None, Some(other_refusal)) => {
                self.refusal = Some(other_refusal.clone());
            }
            _ => {}
        }
    }
}

/// Log probability information for a single token.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Logprob {
    /// The token string.
    pub token: String,
    /// The raw bytes of the token.
    pub bytes: Option<Vec<u8>>,
    /// The log probability of this token.
    pub logprob: rust_decimal::Decimal,
    /// The top alternative tokens and their log probabilities.
    pub top_logprobs: Vec<TopLogprob>,
}

/// A top alternative token with its log probability.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TopLogprob {
    /// The token string.
    pub token: String,
    /// The raw bytes of the token.
    pub bytes: Option<Vec<u8>>,
    /// The log probability of this token.
    pub logprob: Option<rust_decimal::Decimal>,
}
