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
    /// Returns `(reference, files)` where `reference` is a
    /// [`super::function_execution_task_log_reference::LogReference`]
    /// carrying `index`, `task_index`, `task_path`, and optionally
    /// `swiss_pool_index`, `swiss_round`, `split_index`.
    /// Files under `functions/executions/`.
    #[cfg(feature = "filesystem")]
    pub fn produce_files(
        &self,
    ) -> (
        super::function_execution_task_log_reference::LogReference,
        Vec<crate::filesystem::logs::LogFile>,
    ) {
        let (path, files) = match self.inner.produce_files() {
            Some((inner_ref, files)) => (inner_ref.path, files),
            None => (String::new(), Vec::new()),
        };
        let mut reference = super::function_execution_task_log_reference::LogReference::new(
            path,
            self.index,
            self.task_index,
            self.task_path.clone(),
        );
        reference.swiss_pool_index = self.swiss_pool_index;
        reference.swiss_round = self.swiss_round;
        reference.split_index = self.split_index;
        (reference, files)
    }

    /// Delegates to the inner function execution. Returns a boxed
    /// iterator to match the inner's type.
    #[cfg(feature = "filesystem")]
    pub fn produce_message_rows(
        &self,
    ) -> Box<dyn Iterator<Item = crate::filesystem::db::schema::MessageRow> + Send + '_> {
        self.inner.produce_message_rows()
    }
}
