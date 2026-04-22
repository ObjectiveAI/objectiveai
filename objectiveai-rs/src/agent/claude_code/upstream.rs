//! Claude Code upstream types.

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// Claude Code upstream marker.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "agent.claude_code.Upstream")]
pub enum Upstream {
    #[default]
    ClaudeCode,
}
