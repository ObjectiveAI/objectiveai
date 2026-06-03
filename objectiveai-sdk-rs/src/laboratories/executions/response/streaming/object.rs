use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "laboratories.executions.response.streaming.Object")]
pub enum Object {
    #[serde(rename = "laboratory.execution.chunk")]
    LaboratoryExecutionChunk,
}
