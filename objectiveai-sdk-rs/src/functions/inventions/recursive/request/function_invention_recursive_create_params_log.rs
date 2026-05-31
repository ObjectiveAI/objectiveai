//! `FunctionInventionRecursiveCreateParamsLog` — on-disk shape of
//! [`super::FunctionInventionRecursiveCreateParams`].
//!
//! Two fields get extracted to per-leaf files:
//!
//! - `state` (the invention "spec") →
//!   `Option<LogReference>` (`.json` file under
//!   `<route_base>/state/`).
//! - `continuation` → `Option<LogReference>` (`.txt` file under
//!   `<route_base>/continuation/`).
//!
//! Everything else (remote / overwrite / provider / agent / prompt /
//! seed / stream / max_step_retries) stays inline.

use crate::{agent, functions};
use schemars::JsonSchema;
use serde::Serialize;

use crate::filesystem::logs::LogReference;

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[schemars(rename = "functions.inventions.recursive.request.FunctionInventionRecursiveCreateParamsLog")]
pub struct FunctionInventionRecursiveCreateParamsLog {
    pub remote: crate::Remote,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub overwrite: Option<bool>,
    pub state: LogReference,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub provider: Option<agent::completions::request::Provider>,
    pub agent: agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional,
    pub prompt: functions::inventions::prompts::InlinePromptOrRemoteCommitOptional,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub max_step_retries: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub continuation: Option<LogReference>,
}
