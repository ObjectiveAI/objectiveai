use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "laboratories.executions.response.streaming.Object")]
pub enum Object {
    #[serde(rename = "laboratory.execution.chunk")]
    LaboratoryExecutionChunk,
}
