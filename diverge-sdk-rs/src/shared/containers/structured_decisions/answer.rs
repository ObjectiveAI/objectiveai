//! One question's answer.

use indexmap::IndexMap;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// The answer to one [`Question`](super::Question), of the kind the
/// question is. Untagged: the three shapes share no set of members,
/// and each takes no member but its own, so a value is exactly one of
/// them.
///
/// | question | answer |
/// |---|---|
/// | noul | `{"noul": 0.25}` |
/// | choice | `{"probabilities": {"<option value>": 0.7, …}}` |
/// | score | `{"score": 3.4, "probabilities": [0.1, 0.2, …]}` |
///
/// Every number is a [`Decimal`], written as a JSON number and never
/// as a string, as this crate's `rust_decimal` writes one
/// (`serde-float`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(untagged)]
pub enum Answer {
    /// A score's: see [`Score`].
    Score(Score),
    /// A choice's: see [`Choice`].
    Choice(Choice),
    /// A noul's: see [`Noul`].
    Noul(Noul),
}

/// A noul's answer: the one number.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Noul {
    /// The number.
    pub noul: Decimal,
}

/// A choice's answer: a probability for each option, by the option's
/// value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Choice {
    /// Each option's probability, keyed by its value, in the order the
    /// agent's program stated them.
    pub probabilities: IndexMap<String, Decimal>,
}

/// A score's answer: the score, and a probability for each level.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Score {
    /// The score.
    pub score: Decimal,
    /// Each level's probability, in the levels' order.
    pub probabilities: Vec<Decimal>,
}
