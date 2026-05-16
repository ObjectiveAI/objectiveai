use std::borrow::Cow;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error;

/// An inner error from a [`FunctionExecutionChunk`](super::FunctionExecutionChunk).
///
/// Each variant carries `task_path` — the full hierarchical path from the
/// root execution down to the failing task (or, for the reasoning variant,
/// to the execution whose reasoning agent failed). Empty `task_path` means
/// the error is at the root execution itself.
///
/// Wire shape (internally tagged on `"type"`):
/// ```json
/// {"type":"function_task_error","task_path":[0],
///  "swiss_pool_index":1,"swiss_round":0,"error":{}}
/// {"type":"vector_completion_task_error","task_path":[0,1],"error":{}}
/// {"type":"vector_completion_task_error","task_path":[0,1],
///  "agent_completion_index":2,"error":{}}
/// {"type":"reasoning_agent_completion_error","task_path":[],"error":{}}
/// ```
///
/// Does NOT include:
/// - The chunk's own top-level `.error` — reachable directly via
///   [`FunctionExecutionChunk::error`](super::FunctionExecutionChunk::error).
/// - The reasoning summary's own `.error`; only the inner agent
///   completion's failure surfaces as a reasoning error.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "functions.executions.response.streaming.InnerError")]
pub enum InnerError<'a> {
    /// A nested function execution task failed at its top level.
    /// Yielded when a
    /// [`FunctionExecutionTaskChunk`](super::FunctionExecutionTaskChunk)'s
    /// wrapped inner `FunctionExecutionChunk` has its own `.error` set.
    ///
    /// `task_path` locates the failing task; `swiss_pool_index` /
    /// `swiss_round` / `split_index` carry the tournament/split context
    /// from the wrapper when set.
    #[schemars(title = "FunctionTaskError")]
    FunctionTaskError {
        task_path: Vec<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        swiss_pool_index: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        swiss_round: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        split_index: Option<u64>,
        error: Cow<'a, error::ResponseError>,
    },
    /// A vector completion task failed. The optional `agent_completion_index`
    /// is the discriminator:
    /// - `None` → the task itself failed (the wrapper's own `.error`).
    /// - `Some(N)` → agent completion `N` inside the task's vector
    ///   completion failed.
    ///
    /// `task_path` locates the vector completion task.
    #[schemars(title = "VectorCompletionTaskError")]
    VectorCompletionTaskError {
        task_path: Vec<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        agent_completion_index: Option<u64>,
        error: Cow<'a, error::ResponseError>,
    },
    /// The inner agent completion of a reasoning summary failed
    /// (`ReasoningSummaryChunk.inner.error`).
    ///
    /// `task_path` identifies whose reasoning — empty for the root
    /// execution; non-empty for a nested execution at that path.
    #[schemars(title = "ReasoningAgentCompletionError")]
    ReasoningAgentCompletionError {
        task_path: Vec<u64>,
        error: Cow<'a, error::ResponseError>,
    },
}
