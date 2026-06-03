//! On-disk shape of a `FunctionExecutionChunk` log file.
//!
//! Mirrors [`super::FunctionExecutionChunk`]'s field-by-field shape
//! with three type swaps and one order tweak:
//!
//! - `tasks: Vec<TaskChunk>` → `Vec<task_log_reference::LogReference>`
//!   (untagged enum dispatching to function-execution or vector-completion
//!   task references; each task in its own file)
//! - `retry_token: Option<String>` →
//!   `Option<LogReference>` (plain — token extracted to its own file)
//! - `reasoning: Option<ReasoningSummaryChunk>` →
//!   `Option<reasoning_summary_log_reference::LogReference>` (reasoning
//!   extracted to its own file)
//!
//! Field order matches what the legacy `to_value(&shell) +
//! Map::insert` chain produced on disk — specifically, `reasoning`
//! lands at the END of the object (appended via Map::insert),
//! NOT in the middle where the wire chunk's struct declaration puts
//! it. Mirroring that order keeps the snapshot tests byte-identical.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::agent;
use crate::error;
use crate::logs::LogReference;
use crate::functions::executions::response;

use super::{reasoning_summary_log_reference, task_log_reference};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(
    rename = "functions.executions.response.streaming.FunctionExecutionChunkLog"
)]
pub struct FunctionExecutionChunkLog {
    pub id: String,
    pub tasks: Vec<task_log_reference::LogReference>,
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
    pub retry_token: Option<LogReference>,
    pub created: u64,
    pub function: Option<crate::RemotePath>,
    pub profile: Option<crate::RemotePath>,
    pub object: response::streaming::Object,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub usage: Option<agent::completions::response::Usage>,
    /// Appended at the end to match the legacy on-disk order: the
    /// old code inserted `reasoning` via `Map::insert` AFTER all
    /// the shell fields, putting it at the tail of the object.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub reasoning: Option<reasoning_summary_log_reference::LogReference>,
}
