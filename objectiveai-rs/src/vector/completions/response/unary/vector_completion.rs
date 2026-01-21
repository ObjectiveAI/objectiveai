//! Unary (non-streaming) vector completion response.

use crate::vector::completions::response;
use serde::{Deserialize, Serialize};

/// A complete vector completion response (non-streaming).
///
/// Contains the final scores, all votes from the ensemble, and the underlying
/// chat completions that produced those votes.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorCompletion {
    /// Unique identifier for this vector completion.
    pub id: String,
    /// The underlying chat completions from each LLM in the ensemble.
    pub completions: Vec<super::ChatCompletion>,
    /// Individual votes from each LLM, showing their selections.
    pub votes: Vec<response::Vote>,
    /// Final weighted scores for each response option. Sums to 1.
    pub scores: Vec<rust_decimal::Decimal>,
    /// Total weight allocated to each response option. Same length as `scores`.
    /// For discrete votes, an LLM's full weight goes to its selected response.
    /// For probabilistic votes, the weight is divided according to the distribution.
    pub weights: Vec<rust_decimal::Decimal>,
    /// Unix timestamp when the completion was created.
    pub created: u64,
    /// ID of the ensemble used for this completion.
    pub ensemble: String,
    /// Object type identifier (`"vector.completion"`).
    pub object: super::Object,
    /// Aggregated token and cost usage across all completions.
    pub usage: response::Usage,
}

impl From<response::streaming::VectorCompletionChunk> for VectorCompletion {
    fn from(
        response::streaming::VectorCompletionChunk {
            id,
            completions,
            votes,
            scores,
            weights,
            created,
            ensemble,
            object,
            usage,
        }: response::streaming::VectorCompletionChunk,
    ) -> Self {
        Self {
            id,
            completions: completions
                .into_iter()
                .map(super::ChatCompletion::from)
                .collect(),
            votes,
            scores,
            weights,
            created,
            ensemble,
            object: object.into(),
            usage: usage.unwrap_or_default(),
        }
    }
}
