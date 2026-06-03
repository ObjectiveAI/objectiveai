//! Log probability information for generated tokens.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Log probabilities for generated tokens.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Default,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.completions.response.Logprobs")]
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
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Default,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.completions.response.Logprob")]
pub struct Logprob {
    /// The token string.
    pub token: String,
    /// The raw bytes of the token.
    pub bytes: Option<Vec<u8>>,
    /// The log probability of this token.
    #[serde(deserialize_with = "crate::serde_util::decimal")]
    #[schemars(with = "f64")]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_rust_decimal)]
    pub logprob: rust_decimal::Decimal,
    /// The top alternative tokens and their log probabilities.
    pub top_logprobs: Vec<TopLogprob>,
}

/// A top alternative token with its log probability.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Default,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "agent.completions.response.TopLogprob")]
pub struct TopLogprob {
    /// The token string.
    pub token: String,
    /// The raw bytes of the token.
    pub bytes: Option<Vec<u8>>,
    /// The log probability of this token.
    #[serde(deserialize_with = "crate::serde_util::option_decimal")]
    #[schemars(with = "Option<f64>")]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_rust_decimal)]
    pub logprob: Option<rust_decimal::Decimal>,
}
