use crate::{agent, laboratories::executions::response};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// A single builder agent completion within a laboratory execution (non-streaming).
#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
#[schemars(rename = "laboratories.executions.response.unary.Builder")]
pub struct Builder {
    /// Builder index (0-based).
    pub index: u64,
    /// Agent index (0-based).
    pub agent_index: u64,
    #[serde(flatten)]
    pub inner: agent::completions::response::unary::AgentCompletion,
}

impl From<response::streaming::BuilderChunk> for Builder {
    fn from(
        response::streaming::BuilderChunk {
            index,
            agent_index,
            inner,
        }: response::streaming::BuilderChunk,
    ) -> Self {
        Self {
            index,
            agent_index,
            inner: inner.into(),
        }
    }
}
