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
    /// Returns `(reference, files)` where `reference` is a
    /// [`super::vector_completion_task_log_reference::LogReference`]
    /// carrying `index`, `task_index`, `task_path`, and optionally
    /// `error`. Files under `vector/completions/`.
    #[cfg(feature = "filesystem")]
    pub fn produce_files(
        &self,
    ) -> (
        super::vector_completion_task_log_reference::LogReference,
        Vec<crate::filesystem::logs::LogFile>,
    ) {
        let (path, files) = match self.inner.produce_files() {
            Some((inner_ref, files)) => (inner_ref.path, files),
            None => (String::new(), Vec::new()),
        };
        let mut reference = super::vector_completion_task_log_reference::LogReference::new(
            path,
            self.index,
            self.task_index,
            self.task_path.clone(),
        );
        if let Some(error) = &self.error {
            reference.error = Some(serde_json::to_value(error).unwrap());
        }
        (reference, files)
    }

    /// Delegates to the inner vector completion.
    #[cfg(feature = "filesystem")]
    pub fn produce_message_rows(
        &self,
    ) -> impl Iterator<Item = crate::filesystem::db::schema::MessageRow> + Send + '_ {
        self.inner.produce_message_rows()
    }
}
