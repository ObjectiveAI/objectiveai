//! Log probabilities.

use diverge_sdk::provider::endpoints::containers::agents::run::server::response;
use serde::Deserialize;

/// Log probabilities for generated tokens.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Deserialize,
    Default,
)]
pub struct Logprobs {
    /// Log probabilities for content tokens.
    pub content: Option<Vec<Logprob>>,
    /// Log probabilities for refusal tokens.
    pub refusal: Option<Vec<Logprob>>,
}

/// Log probability information for a single token.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Deserialize,
    Default,
)]
pub struct Logprob {
    /// The token string.
    pub token: String,
    /// The raw bytes of the token.
    pub bytes: Option<Vec<u8>>,
    /// The log probability of this token.
    #[serde(deserialize_with = "crate::serde_util::decimal")]
    pub logprob: rust_decimal::Decimal,
    /// The top alternative tokens and their log probabilities.
    pub top_logprobs: Vec<TopLogprob>,
}

/// A top alternative token with its log probability.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Deserialize,
    Default,
)]
pub struct TopLogprob {
    /// The token string.
    pub token: String,
    /// The raw bytes of the token.
    pub bytes: Option<Vec<u8>>,
    /// The log probability of this token.
    #[serde(deserialize_with = "crate::serde_util::option_decimal")]
    pub logprob: Option<rust_decimal::Decimal>,
}

/// Field for field: the shapes agree, because both descend from the
/// same upstream vocabulary.
impl From<Logprob> for response::Logprob {
    fn from(logprob: Logprob) -> Self {
        response::Logprob {
            token: logprob.token,
            bytes: logprob.bytes,
            logprob: logprob.logprob,
            top_logprobs: logprob
                .top_logprobs
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

impl From<TopLogprob> for response::TopLogprob {
    fn from(top: TopLogprob) -> Self {
        response::TopLogprob {
            token: top.token,
            bytes: top.bytes,
            logprob: top.logprob,
        }
    }
}

/// A choice's logprob list, as the form a response chunk carries —
/// converted element-wise, or nothing.
pub(super) fn into_chunk_logprobs(
    logprobs: Option<Vec<Logprob>>,
) -> Option<Vec<response::Logprob>> {
    logprobs.map(|list| list.into_iter().map(Into::into).collect())
}
