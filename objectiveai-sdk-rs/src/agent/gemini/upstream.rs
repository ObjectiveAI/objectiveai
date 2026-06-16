//! Gemini upstream types.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Gemini upstream marker.
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
#[schemars(rename = "agent.gemini.Upstream")]
pub enum Upstream {
    #[default]
    Gemini,
}
