use crate::agent;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// Streaming chunk for a single builder agent completion within a laboratory execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "laboratories.executions.response.streaming.BuilderChunk")]
pub struct BuilderChunk {
    /// Builder index (0-based).
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub index: u64,
    /// Agent index (0-based).
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub agent_index: u64,
    #[serde(flatten)]
    pub inner: agent::completions::response::streaming::AgentCompletionChunk,
}

impl BuilderChunk {
    pub fn push(&mut self, other: &BuilderChunk) {
        self.inner.push(&other.inner);
    }

    /// Produces log files for this builder completion.
    ///
    /// Returns `(reference, files)` where `reference` includes `"index"` and `"agent_index"`.
    /// Files are written under `agent/completions/`.
    #[cfg(feature = "filesystem")]
    pub fn produce_files(&self) -> (serde_json::Value, Vec<crate::filesystem::logs::LogFile>) {
        let (mut reference, files) = match self.inner.produce_files() {
            Some((reference, files)) => (reference, files),
            None => return (serde_json::json!({ "type": "reference", "index": self.index, "agent_index": self.agent_index }), Vec::new()),
        };
        if let Some(map) = reference.as_object_mut() {
            map.insert("index".to_string(), serde_json::json!(self.index));
            map.insert("agent_index".to_string(), serde_json::json!(self.agent_index));
        }
        (reference, files)
    }
}
