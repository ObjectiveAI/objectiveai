//! `ReasoningSummaryLogRef` — ref into `logs.agent_completion_responses`
//! for the function's reasoning-summary slot. Carries the
//! wrapper-level `error` so a reasoning slot that failed before
//! producing an id-bearing chunk still has its failure visible at the
//! parent's ref level.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error;
use crate::logs::LogRef;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(
    rename = "functions.executions.response.streaming.ReasoningSummaryLogRef"
)]
pub struct ReasoningSummaryLogRef {
    #[serde(flatten)]
    pub log_ref: LogRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub error: Option<error::ResponseError>,
}
