//! Per-token log probabilities.

use serde::{Deserialize, Serialize};

/// Log probabilities for the tokens of one turn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Logprobs {
    /// Log probabilities for content tokens.
    pub content: Option<Vec<Logprob>>,
    /// Log probabilities for refusal tokens.
    pub refusal: Option<Vec<Logprob>>,
}

impl Logprobs {
    /// Append another chunk's log probabilities. Tokens accumulate in
    /// arrival order — this is a sequence, so it extends rather than
    /// merging by key.
    pub fn push(&mut self, other: &Logprobs) {
        match (&mut self.content, &other.content) {
            (Some(this), Some(that)) => this.extend(that.clone()),
            (None, Some(that)) => self.content = Some(that.clone()),
            _ => {}
        }
        match (&mut self.refusal, &other.refusal) {
            (Some(this), Some(that)) => this.extend(that.clone()),
            (None, Some(that)) => self.refusal = Some(that.clone()),
            _ => {}
        }
    }
}

/// One token and what the model thought of it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Logprob {
    /// The token as text.
    pub token: String,
    /// The token's raw bytes, when the text is not a faithful
    /// representation of them.
    pub bytes: Option<Vec<u8>>,
    /// The log probability the model assigned it.
    pub logprob: rust_decimal::Decimal,
    /// What the model nearly chose instead.
    pub top_logprobs: Vec<TopLogprob>,
}

/// An alternative the model considered at one position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TopLogprob {
    /// The token as text.
    pub token: String,
    /// The token's raw bytes.
    pub bytes: Option<Vec<u8>>,
    /// Its log probability, when the provider reports one.
    pub logprob: Option<rust_decimal::Decimal>,
}
