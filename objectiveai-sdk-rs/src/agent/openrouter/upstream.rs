//! OpenRouter agent types.

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// OpenRouter upstream marker.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "agent.openrouter.Upstream")]
pub enum Upstream {
    #[default]
    Openrouter,
}
