//! `FunctionExecutionCreateParamsLog` — on-disk shape of
//! [`super::FunctionExecutionCreateParams`].
//!
//! Two fields get extracted to per-leaf files:
//!
//! - `input` → [`crate::functions::expression::InputValueLog`]
//!   (recursive tree of [`LogReference`]s per the input shape; see
//!   `InputValue::extract_to_files`).
//! - `continuation` → `Option<LogReference>` (own `.txt` file under
//!   `<route_base>/continuation/`).
//!
//! Everything else (function / profile / reasoning / strategy /
//! provider / flags / seed) stays inline — small, structurally
//! important for log readability.

use crate::{agent, functions};
use schemars::JsonSchema;
use serde::Serialize;

use crate::LogReference;

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[schemars(
    rename = "functions.executions.request.FunctionExecutionCreateParamsLog"
)]
pub struct FunctionExecutionCreateParamsLog {
    pub function: functions::FullInlineFunctionOrRemoteCommitOptional,
    pub profile: functions::InlineProfileOrRemoteCommitOptional,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub retry_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub from_cache: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub reasoning: Option<super::Reasoning>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub strategy: Option<super::Strategy>,
    pub input: functions::expression::InputValueLog,
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
    pub continuation: Option<LogReference>,
}
