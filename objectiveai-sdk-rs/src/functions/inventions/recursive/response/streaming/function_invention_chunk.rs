use crate::functions;
use crate::agent::completions::response::streaming::AgentCompletionIds;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "functions.inventions.recursive.response.streaming.FunctionInventionChunk")]
pub struct FunctionInventionChunk {
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub index: u64,
    #[serde(flatten)]
    pub inner:
        functions::inventions::response::streaming::FunctionInventionChunk,
}

impl AgentCompletionIds for FunctionInventionChunk {
    fn agent_completion_ids(&self) -> impl Iterator<Item = &str> {
        self.inner.agent_completion_ids()
    }
}

impl FunctionInventionChunk {
    pub fn push(&mut self, other: &FunctionInventionChunk) {
        self.inner.push(&other.inner);
    }

    /// Produces log files for this invention within a recursive invention.
    ///
    /// Returns `(reference, files)` where `reference` is an
    /// [`indexed_reference::LogReference`] carrying `index`. Files
    /// are written under `functions/inventions/`.
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

    /// Delegates to the inner non-recursive invention.
    #[cfg(feature = "filesystem")]
    pub fn produce_message_rows(
        &self,
    ) -> impl Iterator<Item = crate::filesystem::db::schema::MessageRow> + Send + '_ {
        self.inner.produce_message_rows()
    }
}
