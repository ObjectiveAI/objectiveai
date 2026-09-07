//! Per-token log probabilities.
//!
//! There is no `Logprobs` wrapper. The old shape had one so a single
//! chunk could carry `content` and `refusal` token lists side by side;
//! here the chunk's own `type` already says which it is, so a bare
//! list of tokens is the whole of it.

use serde::{Deserialize, Serialize};

/// One token, and what the model thought of it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Logprob {
    /// The token as text.
    pub token: String,
    /// The token's raw bytes, for tokens that are not valid UTF-8 on
    /// their own — a multi-byte character split across two tokens
    /// leaves each half unrepresentable as text.
    pub bytes: Option<Vec<u8>>,
    /// The log probability the model assigned it.
    ///
    /// A decimal, not a float: these round-trip through JSON, and
    /// binary floating point loses exactly the low-order digits that
    /// make two providers' numbers comparable.
    pub logprob: rust_decimal::Decimal,
    /// What the model nearly chose instead, at this position. Empty
    /// when alternatives were not requested.
    pub top_logprobs: Vec<TopLogprob>,
}

/// An alternative the model considered at one position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TopLogprob {
    /// The token as text.
    pub token: String,
    /// The token's raw bytes. See [`Logprob::bytes`].
    pub bytes: Option<Vec<u8>>,
    /// Its log probability, when the provider reports one.
    pub logprob: Option<rust_decimal::Decimal>,
}
