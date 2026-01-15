use crate::functions::profiles::computations::response;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Object {
    #[serde(rename = "function.profile.computation")]
    FunctionProfileComputation,
}

impl From<response::streaming::Object> for Object {
    fn from(value: response::streaming::Object) -> Self {
        match value {
            response::streaming::Object::FunctionProfileComputationChunk => {
                Self::FunctionProfileComputation
            }
        }
    }
}
