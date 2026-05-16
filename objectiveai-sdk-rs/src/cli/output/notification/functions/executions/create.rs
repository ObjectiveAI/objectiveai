use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Result of `functions executions create`.
///
/// Wire: `{"type":"notification","execution":{"output":...}}`.
///
/// Per-task / per-vote / reasoning errors are surfaced live during
/// streaming via `Output::Error` notifications (see the CLI's
/// streaming handler). They are NOT bundled into this payload.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[schemars(rename = "cli.output.notification.functions.executions.Execution")]
pub struct Execution {
    pub execution: ExecutionResult,
}

/// Body of an execution result: the final task output.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[schemars(rename = "cli.output.notification.functions.executions.ExecutionResult")]
pub struct ExecutionResult {
    pub output: crate::functions::expression::TaskOutputOwned,
}
