use crate::agent::completions::response::streaming::AgentCompletionIds;
use crate::{agent, error};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    Default,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(
    rename = "functions.executions.response.streaming.ReasoningSummaryChunk"
)]
pub struct ReasoningSummaryChunk {
    #[serde(flatten)]
    pub inner: agent::completions::response::streaming::AgentCompletionChunk,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub error: Option<error::ResponseError>,
}

impl AgentCompletionIds for ReasoningSummaryChunk {
    fn agent_completion_ids(&self) -> impl Iterator<Item = &str> {
        self.inner.agent_completion_ids()
    }
}

impl ReasoningSummaryChunk {
    pub fn push(&mut self, other: &ReasoningSummaryChunk) {
        self.inner.push(&other.inner);
        if let Some(error) = &other.error {
            self.error = Some(error.clone());
        }
    }

    /// Delegates to the inner agent completion's message-row extractor.
    #[cfg(feature = "filesystem")]
    pub fn produce_message_rows(
        &self,
    ) -> impl Iterator<Item = crate::filesystem::db::schema::MessageRow> + Send + '_
    {
        self.inner.produce_message_rows()
    }
}
