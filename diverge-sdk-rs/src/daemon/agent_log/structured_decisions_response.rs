//! An agent's answers to a structured decision, numbered.

use serde::{Deserialize, Serialize};

use crate::provider::endpoints::containers::agents::run::server::response::StructuredDecisionsResponseChunk;

/// The answers as the agent's program said them, and the id the log
/// gave them: the id of the decision they answer, where they are the
/// first after one, by the rules
/// [`UserStructuredDecisions`](super::UserStructuredDecisions) states.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct StructuredDecisionsResponse {
    /// The log's id for the response.
    pub id: u64,
    /// The answers.
    #[serde(flatten)]
    pub chunk: StructuredDecisionsResponseChunk,
}
