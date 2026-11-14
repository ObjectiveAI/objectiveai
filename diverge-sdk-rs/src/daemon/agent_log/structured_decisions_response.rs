//! An agent's answers to a structured decision, numbered and scored.

use indexmap::IndexMap;
use rmcp::model::MetaObject;
use serde::{Deserialize, Serialize};

use super::Answer;
use crate::provider::endpoints::containers::agents::run::server::response::{StructuredDecisionsResponseChunk, StructuredDecisionsResponseChunkType};

/// The answers the agent's program said, as the log keeps them: the
/// chunk's own members, each answer with what the daemon made of it —
/// a choice its `choice`, a score its `score`: see [`Answer`] — and
/// the id the log gave them: the id of the decision they answer, where
/// they are the first after one, by the rules
/// [`UserStructuredDecisions`](super::UserStructuredDecisions) states.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct StructuredDecisionsResponse {
    /// The log's id for the response.
    pub id: u64,
    pub r#type: StructuredDecisionsResponseChunkType,
    /// Each question's answer, by the question's id, in the order the
    /// agent's program stated them.
    pub decisions: IndexMap<String, Answer>,
    #[serde(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaObject>,
}

impl StructuredDecisionsResponse {
    /// The item for `chunk`, the answers as the agent's program said
    /// them, under the log's `id`: every answer made the log's by
    /// [`Answer::of`], and nothing else of the chunk changed.
    pub fn new(id: u64, chunk: StructuredDecisionsResponseChunk) -> Self {
        Self {
            id,
            r#type: chunk.r#type,
            decisions: chunk.decisions.into_iter().map(|(question, said)| (question, Answer::of(said))).collect(),
            meta: chunk.meta,
        }
    }
}
