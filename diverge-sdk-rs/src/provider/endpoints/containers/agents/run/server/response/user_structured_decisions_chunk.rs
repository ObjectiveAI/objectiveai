//! The user structured decisions chunk.

use rmcp::model::MetaObject;
use serde::{Deserialize, Serialize};

use crate::shared::containers::structured_decisions::Decision;

/// A structured decision of the caller's, landing: the whole
/// [`Decision`] — its key, its content, its questions — as the run it
/// starts takes it, and the first chunk of that run. One chunk, where
/// a chat message is one per content block: a decision is asked whole.
/// The key is what the log credits its sender by, as a user part's is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct UserStructuredDecisionsChunk {
    pub r#type: UserStructuredDecisionsChunkType,
    #[serde(flatten)]
    pub decision: Decision,
    #[serde(rename = "_meta", default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaObject>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[derive(schemars::JsonSchema)]
pub enum UserStructuredDecisionsChunkType {
    #[serde(rename = "user_structured_decisions")]
    #[default]
    UserStructuredDecisions,
}
