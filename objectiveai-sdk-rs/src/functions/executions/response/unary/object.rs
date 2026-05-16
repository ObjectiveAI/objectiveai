use crate::functions::executions::response;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.executions.response.unary.Object")]
pub enum Object {
    #[serde(rename = "scalar.function.execution")]
    ScalarFunctionExecution,
    #[serde(rename = "vector.function.execution")]
    VectorFunctionExecution,
}

impl From<response::streaming::Object> for Object {
    fn from(value: response::streaming::Object) -> Self {
        match value {
            response::streaming::Object::ScalarFunctionExecutionChunk => {
                Object::ScalarFunctionExecution
            }
            response::streaming::Object::VectorFunctionExecutionChunk => {
                Object::VectorFunctionExecution
            }
        }
    }
}
