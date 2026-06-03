use crate::functions::inventions::recursive::response;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.inventions.recursive.response.unary.Object")]
pub enum Object {
    #[serde(rename = "alpha.scalar.function.invention.recursive")]
    AlphaScalarFunctionInventionRecursive,
    #[serde(rename = "alpha.vector.function.invention.recursive")]
    AlphaVectorFunctionInventionRecursive,
}

impl From<response::streaming::Object> for Object {
    fn from(value: response::streaming::Object) -> Self {
        match value {
            response::streaming::Object::AlphaScalarFunctionInventionRecursiveChunk => {
                Object::AlphaScalarFunctionInventionRecursive
            }
            response::streaming::Object::AlphaVectorFunctionInventionRecursiveChunk => {
                Object::AlphaVectorFunctionInventionRecursive
            }
        }
    }
}
