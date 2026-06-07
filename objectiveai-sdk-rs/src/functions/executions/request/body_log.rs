//! `FunctionExecutionCreateParamsLog` — postgres-log shape of
//! [`super::FunctionExecutionCreateParams`].
//!
//! Three fields are extracted to dedicated tables:
//!
//! - `input` → [`LogRef`] into `logs.input` (structured JSON stored
//!   inline as JSONB in that table; content-addressed for dedup).
//! - `retry_token` → `Option<LogRef>` (→ text).
//! - `continuation` → `Option<LogRef>` (→ text).
//!
//! Everything else (function / profile / reasoning / strategy /
//! provider / flags / seed) stays inline — small, structurally
//! important.

use crate::{agent, functions};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::logs::LogRef;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(
    rename = "functions.executions.request.FunctionExecutionCreateParamsLog"
)]
pub struct FunctionExecutionCreateParamsLog {
    pub function: functions::FullInlineFunctionOrRemoteCommitOptional,
    pub profile: functions::InlineProfileOrRemoteCommitOptional,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub retry_token: Option<LogRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub from_cache: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub reasoning: Option<super::Reasoning>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub strategy: Option<super::Strategy>,
    pub input: LogRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub split: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub invert: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub provider: Option<agent::completions::request::Provider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub continuation: Option<LogRef>,
}
