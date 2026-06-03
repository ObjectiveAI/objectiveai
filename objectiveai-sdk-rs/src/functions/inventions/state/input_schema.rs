use crate::functions;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Tagged union of input schema types for invention state serialization.
/// The actual schema is nested under a `schema` key to avoid conflicts
/// with the `type` tag field (since ObjectInputSchema has its own `type` field).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
#[schemars(rename = "functions.inventions.state.InputSchema")]
pub enum InputSchema {
    #[serde(rename = "alpha.scalar.function")]
    #[schemars(title = "ScalarFunctionInputSchema")]
    Scalar {
        schema: functions::alpha_scalar::expression::ScalarFunctionInputSchema,
    },
    #[serde(rename = "alpha.vector.function")]
    #[schemars(title = "VectorFunctionInputSchema")]
    Vector {
        schema: functions::alpha_vector::expression::VectorFunctionInputSchema,
    },
}
