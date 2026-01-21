use crate::functions::executions::response;
use crate::{chat, error};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReasoningSummary {
    #[serde(flatten)]
    pub inner: chat::completions::response::unary::ChatCompletion,
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
