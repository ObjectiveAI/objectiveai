//! Streaming vector completion chunk.

use crate::{agent, vector::completions::response};
use crate::agent::completions::response::streaming::AgentCompletionIds;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// A chunk in a streaming vector completion response.
///
/// Each chunk contains incremental updates to the completion. Use the
/// [`push`](Self::push) method to accumulate chunks into a complete response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "vector.completions.response.streaming.VectorCompletionChunk")]
pub struct VectorCompletionChunk {
    /// Unique identifier for this vector completion.
    pub id: String,
    /// Incremental agent completion chunks from each agent.
    pub completions: Vec<super::AgentCompletionChunk>,
    /// Votes received so far. New votes are appended in subsequent chunks.
    pub votes: Vec<response::Vote>,
    /// Current weighted scores. Updated as new votes arrive.
    #[serde(deserialize_with = "crate::serde_util::vec_decimal")]
    #[schemars(with = "Vec<f64>")]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_vec_rust_decimal)]
    pub scores: Vec<rust_decimal::Decimal>,
    /// Current weight distribution across responses. Updated as new votes arrive.
    #[serde(deserialize_with = "crate::serde_util::vec_decimal")]
    #[schemars(with = "Vec<f64>")]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_vec_rust_decimal)]
    pub weights: Vec<rust_decimal::Decimal>,
    /// Unix timestamp when the completion was created.
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub created: u64,
    /// ID of the swarm used for this completion.
    pub swarm: String,
    /// Object type identifier (`"vector.completion.chunk"`).
    pub object: super::Object,
    /// Aggregated usage statistics. Typically present only in the final chunk.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub usage: Option<agent::completions::response::Usage>,
}

impl AgentCompletionIds for VectorCompletionChunk {
    fn agent_completion_ids(&self) -> impl Iterator<Item = &str> {
        self.completions.iter().flat_map(|c| c.agent_completion_ids())
    }
}

impl VectorCompletionChunk {
    /// Creates a default chunk with uniform scores for the given number of responses.
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
            usage: None,
        }
    }

    /// Yields each inner error from this chunk's per-agent completions,
    /// tagged with the failing completion's `index`.
    ///
    /// Lazy and zero-allocation; collect with `.collect::<Vec<_>>()` if you
    /// need to retain the items past the chunk's lifetime.
    pub fn inner_errors(&self) -> impl Iterator<Item = super::InnerError<'_>> {
        self.completions.iter().filter_map(|c| {
            c.inner.error.as_ref().map(|error| super::InnerError {
                agent_completion_index: c.index,
                error: std::borrow::Cow::Borrowed(error),
            })
        })
    }

    /// Accumulates another chunk into this one.
    ///
    /// Updates scores, weights, and usage, appends new votes.
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
        other_completions: &[super::AgentCompletionChunk],
    ) {
        fn push_completion(
            completions: &mut Vec<super::AgentCompletionChunk>,
            other: &super::AgentCompletionChunk,
        ) {
            fn find_completion(
                completions: &mut Vec<super::AgentCompletionChunk>,
                index: u64,
            ) -> Option<&mut super::AgentCompletionChunk> {
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

    /// Produces the [`LogFile`]s for the log file structure.
    ///
    /// Returns `(reference, files)` where `reference` is a
    /// `{"type": "reference", "path": ...}` JSON value.
    /// All paths are relative to the `logs/` root directory.
    #[cfg(feature = "filesystem")]
    pub fn produce_files(&self) -> Option<(serde_json::Value, Vec<crate::filesystem::logs::LogFile>)> {
        use crate::filesystem::logs::LogFile;
        const ROUTE: &str = "vector/completions";

        let id = &self.id;
        if id.is_empty() {
            return None;
        }

        let mut files: Vec<LogFile> = Vec::new();
        let mut completion_refs: Vec<serde_json::Value> = Vec::new();

        for completion in &self.completions {
            let (reference, completion_files) = completion.produce_files();
            completion_refs.push(reference);
            files.extend(completion_files);
        }

        // Serialize a shell without completions to avoid double-serialization
        let shell = VectorCompletionChunk {
            id: self.id.clone(),
            completions: Vec::new(),
            votes: self.votes.clone(),
            scores: self.scores.clone(),
            weights: self.weights.clone(),
            created: self.created,
            swarm: self.swarm.clone(),
            object: self.object,
            usage: self.usage.clone(),
        };
        let mut root = serde_json::to_value(&shell).unwrap();
        root["completions"] = serde_json::Value::Array(completion_refs);

        let root_file = LogFile {
            route: ROUTE.to_string(),
            id: id.clone(),
            message_index: None,
            media_index: None,
            extension: "json".to_string(),
            content: serde_json::to_vec_pretty(&root).unwrap(),
        };
        let reference = serde_json::json!({ "type": "reference", "path": root_file.path() });
        files.push(root_file);

        Some((reference, files))
    }
}
