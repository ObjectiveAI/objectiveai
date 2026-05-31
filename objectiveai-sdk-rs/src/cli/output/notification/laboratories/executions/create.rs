use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Result of `laboratories executions create`. One item per agent
/// evaluated.
///
/// Wire: `{"type":"notification","laboratory":[...LabResultItem...]}`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[schemars(rename = "cli.output.notification.laboratories.executions.Laboratory")]
pub struct Laboratory {
    pub laboratory: Vec<LabResultItem>,
}

/// One agent evaluated. `score = None` means the evaluation either
/// failed or returned no scoreable output; per-evaluation failures
/// surface live via `Output::Error` notifications during streaming.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[schemars(rename = "cli.output.notification.laboratories.executions.LabResultItem")]
pub struct LabResultItem {
    pub agent: crate::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional,
    pub score: Option<f64>,
}
