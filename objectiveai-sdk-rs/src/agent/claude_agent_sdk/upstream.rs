//! Claude Agent SDK agent types.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Claude Agent SDK upstream marker.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "agent.claude_agent_sdk.Upstream")]
pub enum Upstream {
    #[default]
    ClaudeAgentSdk,
}
