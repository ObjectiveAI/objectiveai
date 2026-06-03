use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "agent.mock.Mode")]
pub enum Mode {
    #[default]
    Default,
    Invention,
    LaboratoryBuilder,
    LaboratoryEvaluation,
}
