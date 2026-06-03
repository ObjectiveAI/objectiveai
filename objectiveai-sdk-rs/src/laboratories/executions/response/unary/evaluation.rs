use crate::{agent, functions, laboratories::executions::response};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A single evaluation agent completion within a laboratory execution (non-streaming).
#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
#[schemars(rename = "laboratories.executions.response.unary.Evaluation")]
pub struct Evaluation {
    /// Evaluation index (0-based).
    pub index: u64,
    /// Agent index (0-based).
    pub agent_index: u64,
    #[serde(flatten)]
    pub inner: agent::completions::response::unary::AgentCompletion,
    pub output: Option<functions::expression::InputValue>,
}

impl From<response::streaming::EvaluationChunk> for Evaluation {
    fn from(
        response::streaming::EvaluationChunk {
            index,
            agent_index,
            inner,
            output,
        }: response::streaming::EvaluationChunk,
    ) -> Self {
        Self {
            index,
            agent_index,
            inner: inner.into(),
            output,
        }
    }
}
