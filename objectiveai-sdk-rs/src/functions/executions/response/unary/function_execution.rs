//! Complete function execution response.

use crate::{
    agent, error,
    functions::{self, executions::response},
};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// A complete function execution response (non-streaming).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.executions.response.unary.FunctionExecution")]
pub struct FunctionExecution {
    /// Unique identifier for this execution.
    pub id: String,
    /// Results from each task in the function.
    pub tasks: Vec<super::Task>,
    /// Whether any tasks encountered errors.
    pub tasks_errors: bool,
    /// Reasoning summary if reasoning was enabled.
    pub reasoning: Option<super::ReasoningSummary>,
    /// The final output (scalar or vector score).
    pub output: super::super::Output,
    /// Error details if the execution failed.
    pub error: Option<error::ResponseError>,
    /// Token for retrying this execution with cached votes.
    pub retry_token: Option<String>,
    /// Unix timestamp when the execution was created.
    pub created: u64,
    /// The function used (if remote).
    pub function: Option<crate::RemotePath>,
    /// The profile used (if remote).
    pub profile: Option<crate::RemotePath>,
    /// Object type identifier.
    pub object: super::Object,
    /// Aggregated token and cost usage.
    pub usage: agent::completions::response::Usage,
}

impl FunctionExecution {
    pub fn any_usage(&self) -> bool {
        self.usage.any_usage()
    }

    /// Normalize non-deterministic fields for test snapshot comparison.
    pub fn normalize_for_tests(&mut self) {
        self.id = String::new();
        self.created = 0;
        self.retry_token = None;
        for task in &mut self.tasks {
            match task {
                super::Task::VectorCompletion(vt) => {
                    // `index` reflects arrival order from concurrent
                    // sub-task streams, which is non-deterministic;
                    // pin it to the local sibling position (the last
                    // element of `task_path`) for snapshot stability.
                    vt.index = vt.task_path.last().copied().unwrap_or(0);
                    vt.inner.normalize_for_tests();
                }
                super::Task::FunctionExecution(ft) => {
                    ft.index = ft.task_path.last().copied().unwrap_or(0);
                    ft.inner.normalize_for_tests();
                }
            }
        }
        self.tasks.sort_by_cached_key(|t| t.snapshot_sort_key());
    }
}

impl From<response::streaming::FunctionExecutionChunk> for FunctionExecution {
    fn from(
        response::streaming::FunctionExecutionChunk {
            id,
            tasks,
            tasks_errors,
            reasoning,
            output,
            error,
            retry_token,
            created,
            function,
            profile,
            object,
            usage,
        }: response::streaming::FunctionExecutionChunk,
    ) -> Self {
        Self {
            id,
            tasks: tasks.into_iter().map(super::Task::from).collect(),
            tasks_errors: tasks_errors.unwrap_or(false),
            reasoning: reasoning.map(super::ReasoningSummary::from),
            output: output.unwrap_or(response::Output {
                output: functions::expression::TaskOutputOwned::Err {
                    error: serde_json::Value::Null,
                },
            }),
            error,
            retry_token,
            created,
            function,
            profile,
            object: object.into(),
            usage: usage.unwrap_or_default(),
        }
    }
}
