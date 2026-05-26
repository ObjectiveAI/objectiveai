use crate::agent::completions::response::streaming::AgentCompletionIds;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, arbitrary::Arbitrary)]
#[schemars(rename = "functions.executions.response.streaming.FunctionExecutionTaskChunk")]
pub struct FunctionExecutionTaskChunk {
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub index: u64,
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub task_index: u64,
    #[arbitrary(with = crate::arbitrary_util::arbitrary_vec_u64)]
    pub task_path: Vec<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_u64)]
    pub swiss_pool_index: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_u64)]
    pub swiss_round: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    #[arbitrary(with = crate::arbitrary_util::arbitrary_option_u64)]
    pub split_index: Option<u64>,
    #[serde(flatten)]
    pub inner: super::FunctionExecutionChunk,
}

impl AgentCompletionIds for FunctionExecutionTaskChunk {
    fn agent_completion_ids(&self) -> impl Iterator<Item = &str> {
        self.inner.agent_completion_ids()
    }
}

impl FunctionExecutionTaskChunk {
    pub fn push(&mut self, other: &super::FunctionExecutionTaskChunk) {
        self.inner.push(&other.inner);
    }

    /// Produces log files for this nested function execution task.
    ///
    /// Returns `(reference, files)` where `reference` includes
    /// `"index"`, `"task_index"`, `"task_path"`, and optionally
    /// `"swiss_pool_index"` and `"swiss_round"`.
    /// Files under `functions/executions/`.
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
            if let Some(v) = self.swiss_pool_index {
                map.insert("swiss_pool_index".to_string(), serde_json::json!(v));
            }
            if let Some(v) = self.swiss_round {
                map.insert("swiss_round".to_string(), serde_json::json!(v));
            }
            if let Some(v) = self.split_index {
                map.insert("split_index".to_string(), serde_json::json!(v));
            }
        }
        (reference, files)
    }
}
