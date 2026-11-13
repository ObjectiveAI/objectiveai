//! What is asked of the agent: a chat message, or a structured
//! decision.

use rmcp::model::ContentBlock;
use serde::{Deserialize, Serialize};

use crate::shared::containers::structured_decisions::Question;

/// What is asked, by its `type`.
///
/// The content of either is a list of MCP content blocks, in order,
/// which the daemon relays to the agent's container verbatim. The
/// daemon reads it first, and holds it to rules before anything is
/// sent on; a request that breaks one reaches nothing — see the
/// [response](super::super::super::server::response::Frame):
///
/// 1. The agent's image declares the kind: `chat`, or
///    `structured_decisions`.
/// 2. A structured decision's questions are valid, as
///    [`validate`](crate::shared::containers::structured_decisions::validate)
///    states.
/// 3. The content carries only what the image says it takes: a chat
///    message's as
///    [`Modalities::lacking`](crate::shared::containers::info::Modalities::lacking)
///    states of the image's `chat.prompt_modalities`; a structured
///    decision's content, instructions, options' descriptions and
///    levels as
///    [`StructuredDecisions::lacking`](crate::shared::containers::info::StructuredDecisions::lacking)
///    states.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    /// A chat message, which joins the agent's conversation.
    Chat {
        /// The message's content, in order.
        content: Vec<ContentBlock>,
    },
    /// A structured decision: a prompt, and the questions asked of it.
    /// The agent's answers are no part of an unwatched request's
    /// answer: they are said in its run, and read in its log.
    StructuredDecisions {
        /// The prompt's content, in order.
        content: Vec<ContentBlock>,
        /// The questions, in order: see [`Question`].
        questions: Vec<Question>,
    },
}
