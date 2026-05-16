use crate::functions::inventions::response;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.inventions.response.unary.Object")]
pub enum Object {
    #[serde(rename = "alpha.scalar.function.invention")]
    AlphaScalarFunctionInvention,
    #[serde(rename = "alpha.vector.function.invention")]
    AlphaVectorFunctionInvention,
}

impl From<response::streaming::Object> for Object {
    fn from(value: response::streaming::Object) -> Self {
        match value {
            response::streaming::Object::AlphaScalarFunctionInventionChunk => {
                Object::AlphaScalarFunctionInvention
            }
            response::streaming::Object::AlphaVectorFunctionInventionChunk => {
                Object::AlphaVectorFunctionInvention
            }
        }
    }
}
