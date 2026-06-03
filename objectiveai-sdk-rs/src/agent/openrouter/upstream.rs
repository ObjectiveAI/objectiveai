//! OpenRouter agent types.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// OpenRouter upstream marker.
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
#[schemars(rename = "agent.openrouter.Upstream")]
pub enum Upstream {
    #[default]
    Openrouter,
}
