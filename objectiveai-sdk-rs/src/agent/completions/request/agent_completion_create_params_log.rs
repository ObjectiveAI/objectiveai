//! `AgentCompletionCreateParamsLog` — postgres-log shape of
//! [`super::AgentCompletionCreateParams`].
//!
//! Field-for-field mirror with these swaps:
//!
//! - `messages: Vec<Message>` → `Vec<MessageLog>` (each message's
//!   content lowered to refs into the content tables).
//! - `response_format: Option<ResponseFormatParam>` — stays inline
//!   (structured small config, no content to extract).
//! - `continuation: Option<String>` → `Option<LogRef>` (→ text).
//!
//! Other small-scalar fields (`provider`, `agent`, `seed`, `stream`)
//! stay inline.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::agent::completions::message::MessageLog;
use crate::logs::LogRef;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "agent.completions.request.AgentCompletionCreateParamsLog")]
pub struct AgentCompletionCreateParamsLog {
    pub messages: Vec<MessageLog>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub provider: Option<super::Provider>,
    pub agent: crate::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub response_format: Option<super::ResponseFormatParam>,
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
