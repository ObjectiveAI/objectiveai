//! Weights for swarm agent influence.
//!
//! Weights specify how much influence each agent in a Swarm has when
//! combining votes into final scores. Can either be a simple vector
//! of decimal weights or a vector of entries that also include an optional
//! `invert` flag, which inverts that agent's vote distribution before it is
//! combined.

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An entry in weights with an explicit weight and optional invert flag.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "WeightsEntry")]
pub struct WeightsEntry {
    /// The weight for this agent in the swarm. Must be in [0, 1].
    #[serde(deserialize_with = "crate::serde_util::decimal")]
    #[schemars(with = "f64")]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_rust_decimal)]
    pub weight: Decimal,
    /// If true, invert this agent's vote distribution before combining.
    ///
    /// When omitted or false, the vote distribution is used as-is.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub invert: Option<bool>,
}

/// Weights for a swarm's agents.
///
/// - `Weights(Vec<Decimal>)` - simple representation (no inversion)
/// - `Entries(Vec<WeightsEntry>)` - weights with optional per-agent `invert`
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(untagged)]
#[schemars(rename = "Weights")]
pub enum Weights {
    /// Simple vector of decimal weights.
    #[schemars(title = "Weights")]
    Weights(
        #[serde(deserialize_with = "crate::serde_util::vec_decimal")]
        #[schemars(with = "Vec<f64>")]
        #[arbitrary(with = crate::arbitrary_util::arbitrary_vec_rust_decimal)]
        Vec<Decimal>,
    ),
    /// Vector of entries with optional invert flags.
    #[schemars(title = "Entries")]
    Entries(Vec<WeightsEntry>),
}

impl Weights {
    /// Returns the length of the underlying weights vector.
    pub fn len(&self) -> usize {
        match self {
            Weights::Weights(weights) => weights.len(),
            Weights::Entries(entries) => entries.len(),
        }
    }

    /// Returns true if the weights contain no entries.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Normalizes into `(weight, invert)` pairs.
    ///
    /// For the `Weights` variant, all `invert` flags are `false`.
    pub fn to_weights_and_invert(&self) -> Vec<(Decimal, bool)> {
        match self {
            Weights::Weights(weights) => {
                weights.iter().map(|w| (*w, false)).collect()
            }
            Weights::Entries(entries) => entries
                .iter()
                .map(|entry| (entry.weight, entry.invert.unwrap_or(false)))
                .collect(),
        }
    }
}
