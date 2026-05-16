//! Codex SDK upstream types.

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// Codex SDK upstream marker.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "agent.codex_sdk.Upstream")]
pub enum Upstream {
    #[default]
    CodexSdk,
}
