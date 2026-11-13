//! The structured decisions response chunk.

use indexmap::IndexMap;
use rmcp::model::MetaObject;
use serde::{Deserialize, Serialize};

use crate::shared::containers::structured_decisions::Answer;

/// The agent's answers to a structured decision: one
/// [`Answer`] for each question, keyed by the question's id, each of
/// the kind its question is. Whole, never a delta: a decision is
/// answered at once.
///
/// Nothing on the wire ties it to the decision it answers. It answers
/// the one whose run it is said in, and the daemon's log numbers the
/// two alike: see
/// [`agent_log`](crate::daemon::agent_log).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct StructuredDecisionsResponseChunk {
    pub r#type: StructuredDecisionsResponseChunkType,
    /// Each question's answer, by the question's id.
    pub decisions: IndexMap<String, Answer>,
    #[serde(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaObject>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[derive(schemars::JsonSchema)]
pub enum StructuredDecisionsResponseChunkType {
    #[serde(rename = "structured_decisions_response")]
    #[default]
    StructuredDecisionsResponse,
}
