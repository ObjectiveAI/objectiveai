use crate::{agent, error};
use crate::agent::completions::response::streaming::AgentCompletionIds;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "functions.executions.response.streaming.ReasoningSummaryChunk")]
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

    /// Produces log files for this reasoning summary.
    ///
    /// Returns `(reference, files)` where `reference` is a
    /// [`super::reasoning_summary_log_reference::LogReference`]
    /// carrying the wrapper's own optional `error`. Files under
    /// `agent/completions/`.
    #[cfg(feature = "filesystem")]
    pub fn produce_files(
        &self,
    ) -> (super::reasoning_summary_log_reference::LogReference, Vec<crate::filesystem::logs::LogFile>) {
        let (path, files) = match self.inner.produce_files() {
            Some((inner_ref, files)) => (inner_ref.path, files),
            None => (String::new(), Vec::new()),
        };
        let mut reference = super::reasoning_summary_log_reference::LogReference::new(path);
        if let Some(error) = &self.error {
            reference.error = Some(serde_json::to_value(error).unwrap());
        }
        (reference, files)
    }

    /// Delegates to the inner agent completion's message-row extractor.
    #[cfg(feature = "filesystem")]
    pub fn produce_message_rows(
        &self,
    ) -> impl Iterator<Item = crate::filesystem::db::schema::MessageRow> + Send + '_ {
        self.inner.produce_message_rows()
    }
}
