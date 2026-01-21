use crate::{chat, error};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReasoningSummaryChunk {
    #[serde(flatten)]
    pub inner: chat::completions::response::streaming::ChatCompletionChunk,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<error::ResponseError>,
}

impl ReasoningSummaryChunk {
    pub fn push(&mut self, other: &ReasoningSummaryChunk) {
        self.inner.push(&other.inner);
        match (&mut self.error, &other.error) {
            (None, Some(other_error)) => {
                self.error = Some(other_error.clone());
            }
            _ => {}
        }
    }
}
