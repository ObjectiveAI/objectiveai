//! Streaming vector completion chunk.

use crate::vector::completions::response;
use serde::{Deserialize, Serialize};

/// A chunk in a streaming vector completion response.
///
/// Each chunk contains incremental updates to the completion. Use the
/// [`push`](Self::push) method to accumulate chunks into a complete response.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorCompletionChunk {
    /// Unique identifier for this vector completion.
    pub id: String,
    /// Incremental chat completion chunks from each LLM.
    pub completions: Vec<super::ChatCompletionChunk>,
    /// Votes received so far. New votes are appended in subsequent chunks.
    pub votes: Vec<response::Vote>,
    /// Current weighted scores. Updated as new votes arrive.
    pub scores: Vec<rust_decimal::Decimal>,
    /// Current weight distribution across responses. Updated as new votes arrive.
    pub weights: Vec<rust_decimal::Decimal>,
    /// Unix timestamp when the completion was created.
    pub created: u64,
    /// ID of the ensemble used for this completion.
    pub ensemble: String,
    /// Object type identifier (`"vector.completion.chunk"`).
    pub object: super::Object,
    /// Aggregated usage statistics. Typically present only in the final chunk.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<response::Usage>,
}

impl VectorCompletionChunk {
    pub fn push(
        &mut self,
        VectorCompletionChunk {
            completions,
            votes,
            scores,
            weights,
            usage,
            ..
        }: &VectorCompletionChunk,
    ) {
        self.push_completions(completions);
        self.votes.extend_from_slice(votes);
        self.scores = scores.clone();
        self.weights = weights.clone();
        match (&mut self.usage, usage) {
            (Some(self_usage), Some(other_usage)) => {
                self_usage.push(other_usage);
            }
            (None, Some(other_usage)) => {
                self.usage = Some(other_usage.clone());
            }
            _ => {}
        }
    }

    fn push_completions(
        &mut self,
        other_completions: &[super::ChatCompletionChunk],
    ) {
        fn push_completion(
            completions: &mut Vec<super::ChatCompletionChunk>,
            other: &super::ChatCompletionChunk,
        ) {
            fn find_completion(
                completions: &mut Vec<super::ChatCompletionChunk>,
                index: u64,
            ) -> Option<&mut super::ChatCompletionChunk> {
                for completion in completions {
                    if completion.index == index {
                        return Some(completion);
                    }
                }
                None
            }
            if let Some(completion) = find_completion(completions, other.index)
            {
                completion.push(other);
            } else {
                completions.push(other.clone());
            }
        }
        for other_completion in other_completions {
            push_completion(&mut self.completions, other_completion);
        }
    }
}
