//! `FunctionExecutionChunkLog` — postgres-log shape of
//! [`super::FunctionExecutionChunk`].
//!
//! Mirrors the wire chunk with three swaps:
//!
//! - `tasks: Vec<TaskChunk>` →
//!   `Vec<`[`super::TaskLogRef`]`>` — each task slot becomes a typed
//!   ref into either `logs.function_execution_responses` or
//!   `logs.vector_completion_responses`.
//! - `reasoning: Option<ReasoningSummaryChunk>` →
//!   `Option<`[`super::ReasoningSummaryLogRef`]`>` — the embedded
//!   agent completion moves into `logs.agent_completion_responses`,
//!   referenced here by id.
//! - `retry_token: Option<String>` → `Option<LogRef>` (→ text).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::agent;
use crate::error;
use crate::functions::executions::response;
use crate::logs::LogRef;

use super::{ReasoningSummaryLogRef, TaskLogRef};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(
    rename = "functions.executions.response.streaming.FunctionExecutionChunkLog"
)]
pub struct FunctionExecutionChunkLog {
    pub id: String,
    pub tasks: Vec<TaskLogRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub tasks_errors: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub output: Option<response::Output>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub error: Option<error::ResponseError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub retry_token: Option<LogRef>,
    pub created: u64,
    pub function: Option<crate::RemotePath>,
    pub profile: Option<crate::RemotePath>,
    pub object: response::streaming::Object,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub usage: Option<agent::completions::response::Usage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub reasoning: Option<ReasoningSummaryLogRef>,
}
