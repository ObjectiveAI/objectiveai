//! One structured decision, as it is handed to an agent.

use rmcp::model::ContentBlock;
use serde::{Deserialize, Serialize};

use super::Question;

/// One structured decision: the key, the content, and the questions.
/// It is one type from the caller to the agent's program: an
/// [`enqueue`](super::enqueue) carries it, and so does the run the
/// proxy starts with it inside the container
/// ([`run`](crate::container_proxy::inside::agent::run)).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Decision {
    /// The caller's handle on the decision: what the chunk that marks
    /// its start carries back, so the log credits its sender. Not read
    /// by the provider or the proxy, which carry it and nothing else.
    pub key: String,
    /// The prompt's content, in order, as a chat message's is.
    pub content: Vec<ContentBlock>,
    /// The questions, in order: see [`Question`].
    pub questions: Vec<Question>,
}
