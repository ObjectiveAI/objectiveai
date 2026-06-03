use crate::laboratories::executions::response;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "laboratories.executions.response.unary.Object")]
pub enum Object {
    #[serde(rename = "laboratory.execution")]
    LaboratoryExecution,
}

impl From<response::streaming::Object> for Object {
    fn from(value: response::streaming::Object) -> Self {
        match value {
            response::streaming::Object::LaboratoryExecutionChunk => {
                Object::LaboratoryExecution
            }
        }
    }
}
