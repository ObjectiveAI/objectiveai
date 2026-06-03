use crate::agent::completions::response::streaming::AgentCompletionIds;
use crate::{agent, functions};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Streaming chunk for a single evaluation agent completion within a laboratory execution.
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
    rename = "laboratories.executions.response.streaming.EvaluationChunk"
)]
pub struct EvaluationChunk {
    /// Evaluation index (0-based).
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub index: u64,
    /// Agent index (0-based).
    #[arbitrary(with = crate::arbitrary_util::arbitrary_u64)]
    pub agent_index: u64,
    #[serde(flatten)]
    pub inner: agent::completions::response::streaming::AgentCompletionChunk,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub output: Option<functions::expression::InputValue>,
}

impl AgentCompletionIds for EvaluationChunk {
    fn agent_completion_ids(&self) -> impl Iterator<Item = &str> + Send {
        self.inner.agent_completion_ids()
    }
}

impl EvaluationChunk {
    pub fn push(&mut self, other: &EvaluationChunk) {
        self.inner.push(&other.inner);
        if let Some(output) = &other.output {
            self.output = Some(output.clone());
        }
    }
}
