use crate::agent;
use crate::agent::completions::response::streaming::AgentCompletionIds;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "functions.inventions.response.streaming.AgentCompletionChunk")]
pub struct AgentCompletionChunk {
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub index: u64,
    #[serde(flatten)]
    pub inner: agent::completions::response::streaming::AgentCompletionChunk,
}

impl AgentCompletionIds for AgentCompletionChunk {
    fn agent_completion_ids(&self) -> impl Iterator<Item = &str> {
        self.inner.agent_completion_ids()
    }
}

impl AgentCompletionChunk {
    pub fn push(&mut self, other: &AgentCompletionChunk) {
        self.inner.push(&other.inner);
    }

    /// Produces log files for this agent completion within a function invention.
    ///
    /// Returns `(reference, files)` where `reference` is an
    /// [`indexed_reference::LogReference`] carrying `index`. Files
    /// are written under `agent/completions/`.
    ///
    /// [`indexed_reference::LogReference`]: crate::filesystem::logs::indexed_reference::LogReference
    #[cfg(feature = "filesystem")]
    pub fn produce_files(
        &self,
    ) -> (
        crate::filesystem::logs::indexed_reference::LogReference,
        Vec<crate::filesystem::logs::LogFile>,
    ) {
        let (path, files) = match self.inner.produce_files() {
            Some((inner_ref, files)) => (inner_ref.path, files),
            None => (String::new(), Vec::new()),
        };
        (
            crate::filesystem::logs::indexed_reference::LogReference::new(path, self.index),
            files,
        )
    }

    /// Delegates to the inner agent completion's message-row extractor.
    #[cfg(feature = "filesystem")]
    pub fn produce_message_rows(
        &self,
    ) -> impl Iterator<Item = crate::filesystem::db::schema::MessageRow> + Send + '_ {
        self.inner.produce_message_rows()
    }
}
