//! `AgentCompletionCreateParamsLog` — on-disk shape of
//! [`super::AgentCompletionCreateParams`].
//!
//! Three fields get extracted to per-leaf files via
//! [`LogReference`]s; everything else stays inline:
//!
//! - `messages` → `Vec<LogReference>` (each ref points at a
//!   per-message file under `<route_base>/messages/<id>-<idx>.json`
//!   holding a [`super::super::message::MessageLog`]).
//! - `response_format` → `Option<LogReference>` (own
//!   `.json` file under `<route_base>/response_format/`).
//! - `continuation` → `Option<LogReference>` (own `.txt` file
//!   under `<route_base>/continuation/`).
//!
//! `provider`, `agent`, `seed`, `stream` stay inline — they're
//! small + structurally important for log-readability.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::logs::LogReference;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.completions.request.AgentCompletionCreateParamsLog")]
pub struct AgentCompletionCreateParamsLog {
    pub messages: Vec<LogReference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub provider: Option<super::Provider>,
    pub agent: crate::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub response_format: Option<LogReference>,
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
