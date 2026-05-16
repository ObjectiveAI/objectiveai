use crate::functions;
use crate::functions::inventions::recursive::response;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.inventions.recursive.response.unary.FunctionInvention")]
pub struct FunctionInvention {
    pub index: u64,
    #[serde(flatten)]
    pub inner: functions::inventions::response::unary::FunctionInvention,
}

impl From<response::streaming::FunctionInventionChunk> for FunctionInvention {
    fn from(
        response::streaming::FunctionInventionChunk { index, inner }: response::streaming::FunctionInventionChunk,
    ) -> Self {
        Self {
            index,
            inner: inner.into(),
        }
    }
}
