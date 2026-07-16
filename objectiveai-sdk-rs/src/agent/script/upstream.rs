//! Script agent types.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Script upstream marker.
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
#[schemars(rename = "agent.script.Upstream")]
pub enum Upstream {
    #[default]
    Script,
}
