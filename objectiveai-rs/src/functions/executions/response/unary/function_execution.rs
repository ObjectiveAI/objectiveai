//! Complete function execution response.

use crate::{
    error,
    functions::{self, executions::response},
    vector,
};
use serde::{Deserialize, Serialize};

/// A complete function execution response (non-streaming).
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub output: functions::expression::FunctionOutput,
    /// Error details if the execution failed.
    pub error: Option<error::ResponseError>,
    /// Token for retrying this execution with cached votes.
    pub retry_token: Option<String>,
    /// Unix timestamp when the execution was created.
    pub created: u64,
    /// ID of the function used (if remote).
    pub function: Option<String>,
    /// ID of the profile used (if remote).
    pub profile: Option<String>,
    /// Object type identifier.
    pub object: super::Object,
    /// Aggregated token and cost usage.
    pub usage: vector::completions::response::Usage,
}

impl FunctionExecution {
    pub fn any_usage(&self) -> bool {
        self.usage.any_usage()
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
            output: output.unwrap_or(
                functions::expression::FunctionOutput::Err(
                    serde_json::Value::Null,
                ),
            ),
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
