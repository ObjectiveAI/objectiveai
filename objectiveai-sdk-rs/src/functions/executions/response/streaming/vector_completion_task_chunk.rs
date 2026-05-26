use crate::{error, vector};
use crate::agent::completions::response::streaming::AgentCompletionIds;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "functions.executions.response.streaming.VectorCompletionTaskChunk")]
pub struct VectorCompletionTaskChunk {
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub index: u64,
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub task_index: u64,
    #[arbitrary(with = crate::arbitrary_util::arbitrary_vec_u64)]
    pub task_path: Vec<u64>,
    #[serde(flatten)]
    pub inner: vector::completions::response::streaming::VectorCompletionChunk,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub error: Option<error::ResponseError>,
}

impl AgentCompletionIds for VectorCompletionTaskChunk {
    fn agent_completion_ids(&self) -> impl Iterator<Item = &str> {
        self.inner.agent_completion_ids()
    }
}

impl VectorCompletionTaskChunk {
    pub fn push(&mut self, other: &VectorCompletionTaskChunk) {
        self.inner.push(&other.inner);
        if let Some(error) = &other.error {
            self.error = Some(error.clone());
        }
    }

    /// Produces log files for this vector completion task.
    ///
    /// Returns `(reference, files)` where `reference` includes
    /// `"index"`, `"task_index"`, `"task_path"`, and optionally `"error"`.
    /// Files under `vector/completions/`.
    #[cfg(feature = "filesystem")]
    pub fn produce_files(&self) -> (serde_json::Value, Vec<crate::filesystem::logs::LogFile>) {
        let (mut reference, files) = match self.inner.produce_files() {
            Some((reference, files)) => (reference, files),
            None => return (serde_json::json!({ "type": "reference", "index": self.index, "task_index": self.task_index, "task_path": self.task_path }), Vec::new()),
        };
        if let Some(map) = reference.as_object_mut() {
            map.insert("index".to_string(), serde_json::json!(self.index));
            map.insert("task_index".to_string(), serde_json::json!(self.task_index));
            map.insert("task_path".to_string(), serde_json::json!(self.task_path));
            if let Some(error) = &self.error {
                map.insert("error".to_string(), serde_json::to_value(error).unwrap());
            }
        }
        (reference, files)
    }
}
