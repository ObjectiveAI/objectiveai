use crate::functions::executions::response;
use crate::{agent, error};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
#[schemars(rename = "functions.executions.response.unary.ReasoningSummary")]
pub struct ReasoningSummary {
    #[serde(flatten)]
    pub inner: agent::completions::response::unary::AgentCompletion,
    pub error: Option<error::ResponseError>,
}

impl From<response::streaming::ReasoningSummaryChunk> for ReasoningSummary {
    fn from(
        response::streaming::ReasoningSummaryChunk {
            inner,
            error,
        }: response::streaming::ReasoningSummaryChunk,
    ) -> Self {
        Self {
            inner: inner.into(),
            error,
        }
    }
}
