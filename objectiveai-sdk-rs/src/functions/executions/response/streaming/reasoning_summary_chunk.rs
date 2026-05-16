use crate::{agent, error};
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

impl ReasoningSummaryChunk {
    pub fn push(&mut self, other: &ReasoningSummaryChunk) {
        self.inner.push(&other.inner);
        if let Some(error) = &other.error {
            self.error = Some(error.clone());
        }
    }

    /// Produces log files for this reasoning summary.
    ///
    /// Returns `(reference, files)`. Files under `agent/completions/`.
    #[cfg(feature = "filesystem")]
    pub fn produce_files(&self) -> (serde_json::Value, Vec<crate::filesystem::logs::LogFile>) {
        let (mut reference, files) = match self.inner.produce_files() {
            Some((reference, files)) => (reference, files),
            None => return (serde_json::json!({ "type": "reference" }), Vec::new()),
        };
        if let Some(error) = &self.error {
            if let Some(map) = reference.as_object_mut() {
                map.insert("error".to_string(), serde_json::to_value(error).unwrap());
            }
        }
        (reference, files)
    }
}
