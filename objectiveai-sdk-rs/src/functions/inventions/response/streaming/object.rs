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
#[schemars(rename = "functions.inventions.response.streaming.Object")]
pub enum Object {
    #[serde(rename = "alpha.scalar.function.invention.chunk")]
    AlphaScalarFunctionInventionChunk,
    #[serde(rename = "alpha.vector.function.invention.chunk")]
    AlphaVectorFunctionInventionChunk,
}
