//! Unary (non-streaming) vector completion response.

use crate::{agent, vector::completions::response};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// A complete vector completion response (non-streaming).
///
/// Contains the final scores, all votes from the swarm, and the underlying
/// agent completions that produced those votes.
#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
#[schemars(rename = "vector.completions.response.unary.VectorCompletion")]
pub struct VectorCompletion {
    /// Unique identifier for this vector completion.
    pub id: String,
    /// The underlying agent completions from each agent in the swarm.
    pub completions: Vec<super::AgentCompletion>,
    /// Individual votes from each agent, showing their selections.
    pub votes: Vec<response::Vote>,
    /// Final weighted scores for each response option. Sums to 1.
    #[serde(deserialize_with = "crate::serde_util::vec_decimal")]
    #[schemars(with = "Vec<f64>")]
    pub scores: Vec<rust_decimal::Decimal>,
    /// Total weight allocated to each response option. Same length as `scores`.
    /// For discrete votes, an LLM's full weight goes to its selected response.
    /// For probabilistic votes, the weight is divided according to the distribution.
    #[serde(deserialize_with = "crate::serde_util::vec_decimal")]
    #[schemars(with = "Vec<f64>")]
    pub weights: Vec<rust_decimal::Decimal>,
    /// Unix timestamp when the completion was created.
    pub created: u64,
    /// ID of the swarm used for this completion.
    pub swarm: String,
    /// Object type identifier (`"vector.completion"`).
    pub object: super::Object,
    /// Aggregated token and cost usage across all completions.
    pub usage: agent::completions::response::Usage,
}

impl VectorCompletion {
    /// Normalize non-deterministic fields for test snapshot comparison.
    pub fn normalize_for_tests(&mut self) {
        self.id = String::new();
        self.created = 0;
        for completion in &mut self.completions {
            completion.inner.normalize_for_tests();
        }
        self.votes.sort_by_key(|v| v.flat_swarm_index);

        // sort completions by JSON representation since ordering is non-determinstic
        self.completions.sort_by_cached_key(|c| serde_json::to_string(&c.inner).unwrap());

        // re-apply completion indices since indices are non-deterministic
        let mut i = 0;
        for completion in &mut self.completions {
            completion.index = i;
            i += 1;
        }
    }

    /// Creates a default completion with uniform scores for the given number of responses.
    pub fn default_from_request_responses_len(
        request_responses_len: usize,
    ) -> Self {
        let weights = vec![rust_decimal::Decimal::ZERO; request_responses_len];
        let scores =
            vec![
                rust_decimal::Decimal::ONE
                    / rust_decimal::Decimal::from(request_responses_len);
                request_responses_len
            ];
        Self {
            id: String::new(),
            completions: Vec::new(),
            votes: Vec::new(),
            scores,
            weights,
            created: 0,
            swarm: String::new(),
            object: super::Object::default(),
            usage: agent::completions::response::Usage::default(),
        }
    }
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
            swarm,
            object,
            usage,
        }: response::streaming::VectorCompletionChunk,
    ) -> Self {
        Self {
            id,
            completions: completions
                .into_iter()
                .map(super::AgentCompletion::from)
                .collect(),
            votes,
            scores,
            weights,
            created,
            swarm,
            object: object.into(),
            usage: usage.unwrap_or_default(),
        }
    }
}
