use std::borrow::Cow;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error;

/// An inner error from a [`LaboratoryExecutionChunk`](super::LaboratoryExecutionChunk).
///
/// Yielded by [`LaboratoryExecutionChunk::inner_errors`](super::LaboratoryExecutionChunk::inner_errors).
/// The variant identifies which child collection the failing agent
/// completion came from; fields locate it by `index` (builder or
/// evaluation index) and `agent_index` (agent slot within that builder
/// or evaluation), with the underlying [`ResponseError`](error::ResponseError)
/// held as a `Cow` so iteration is zero-allocation while deserialization
/// still produces a self-owning value.
///
/// Wire shape (internally tagged on `"type"`):
/// ```json
/// { "type": "builder",    "builder_index": 0,    "agent_completion_index": 1, "error": { } }
/// { "type": "evaluation", "evaluation_index": 2, "agent_completion_index": 0, "error": { } }
/// ```
///
/// Does NOT include the lab chunk's own top-level `.error` — that
/// field is the chunk's own failure state and is reachable directly
/// via [`LaboratoryExecutionChunk::error`](super::LaboratoryExecutionChunk::error).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "laboratories.executions.response.streaming.InnerError")]
pub enum InnerError<'a> {
    /// An error from a [`BuilderChunk`](super::BuilderChunk).
    #[schemars(title = "Builder")]
    Builder {
        /// Builder index (matches `BuilderChunk::index`).
        builder_index: u64,
        /// Agent completion index within the builder (matches `BuilderChunk::agent_index`).
        agent_completion_index: u64,
        /// The underlying error from the agent completion.
        error: Cow<'a, error::ResponseError>,
    },
    /// An error from an [`EvaluationChunk`](super::EvaluationChunk).
    #[schemars(title = "Evaluation")]
    Evaluation {
        /// Evaluation index (matches `EvaluationChunk::index`).
        evaluation_index: u64,
        /// Agent completion index within the evaluation (matches `EvaluationChunk::agent_index`).
        agent_completion_index: u64,
        /// The underlying error from the agent completion.
        error: Cow<'a, error::ResponseError>,
    },
}
