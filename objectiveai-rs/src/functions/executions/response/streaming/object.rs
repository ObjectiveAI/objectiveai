use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Object {
    #[serde(rename = "scalar.function.execution.chunk")]
    ScalarFunctionExecutionChunk,
    #[serde(rename = "vector.function.execution.chunk")]
    VectorFunctionExecutionChunk,
}
