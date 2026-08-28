//! A message in an OpenRouter request body.

use diverge_provider_sdk::endpoints::agentic_loop::run::server::response::AgenticLoopChunk;
use serde::Serialize;

use crate::continuation::{Continuation, ContinuationItem};

use super::{
    AssistantMessage, DeveloperMessage, SystemMessage, ToolMessage,
    UserMessage,
};

/// A message in an OpenRouter request body.
///
/// Tagged by `role`, so each variant serializes to
/// `{"role": ..., ...}` with the variant's own fields beside the tag.
/// The agent's system prompt is the leading `system` or `developer`
/// entry; the conversation follows as `user`, `assistant` and `tool`
/// messages.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "role")]
pub enum Message {
    /// The agent's system prompt, as a system message.
    #[serde(rename = "system")]
    System(SystemMessage),
    /// The agent's system prompt, as a developer message.
    #[serde(rename = "developer")]
    Developer(DeveloperMessage),
    /// A user message from the end user.
    #[serde(rename = "user")]
    User(UserMessage),
    /// An assistant message (model's previous response).
    #[serde(rename = "assistant")]
    Assistant(AssistantMessage),
    /// A tool message containing the result of a tool call.
    #[serde(rename = "tool")]
    Tool(ToolMessage),
}

/// The whole conversation, in the order it happened: the system
/// prompt, then the history the continuation carries — each turn's
/// prompt and what the loop said back — then this turn's prompt,
/// when there is one: turns after the first send none.
///
/// Consecutive assistant chunks always merge into one assistant
/// message; a tool response, or the next prompt, is what ends a run
/// of them. Bookkeeping chunks — usage, notifications, continuation
/// tokens — say nothing conversational and are ignored.
pub fn messages(
    system_prompt: Option<String>,
    continuation: Option<Continuation>,
    prompt: Vec<rmcp::model::ContentBlock>,
) -> Vec<Message> {
    let mut messages = Vec::new();
    let mut current: Option<super::AssistantMessage> = None;

    if let Some(content) = system_prompt {
        messages.push(Message::System(super::SystemMessage::new(content)));
    }

    for item in continuation.into_iter().flat_map(|history| history.0) {
        match item {
            ContinuationItem::Prompt(blocks) => {
                if let Some(assistant) = current.take() {
                    messages.push(Message::Assistant(assistant));
                }
                messages.push(Message::User(super::UserMessage::new(blocks)));
            }
            ContinuationItem::Chunk(chunk) => match chunk {
                AgenticLoopChunk::ToolResponse(_) => {
                    if let Some(assistant) = current.take() {
                        messages.push(Message::Assistant(assistant));
                    }
                    messages
                        .push(Message::Tool(super::ToolMessage::new(chunk)));
                }
                AgenticLoopChunk::Usage(_)
                | AgenticLoopChunk::Notification(_)
                | AgenticLoopChunk::Continuation(_) => {}
                chunk => match &mut current {
                    Some(assistant) => assistant.push(chunk),
                    None => {
                        current = Some(super::AssistantMessage::new(chunk));
                    }
                },
            },
        }
    }

    if let Some(assistant) = current.take() {
        messages.push(Message::Assistant(assistant));
    }
    // Turns after the first send no new prompt — it already rode into
    // the history — and an empty user message is not a message.
    if !prompt.is_empty() {
        messages.push(Message::User(super::UserMessage::new(prompt)));
    }
    messages
}
