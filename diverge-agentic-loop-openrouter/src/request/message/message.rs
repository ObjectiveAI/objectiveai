//! A message in an OpenRouter request body.

use diverge_provider_sdk::endpoints::containers::agents::run::server::response::AgenticLoopChunk;
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
/// of them. Bookkeeping chunks — usage, notifications, and the
/// `user` chunks whose storage form is the `Prompt` item — say
/// nothing extra here and are ignored.
///
/// # Where a prompt lands depends on what it follows
///
/// The continuation stores every prompt the same way — a bare
/// `Prompt` item — and THIS derivation decides what it becomes,
/// which is how steered messages take the position Claude Code
/// gives them:
///
/// - a run of prompts directly behind a tool response folds INTO
///   that tool message, joined by blank lines inside one
///   `system-reminder` section — see
///   [`ToolMessage::fold_steer`](super::ToolMessage::fold_steer);
/// - a run of prompts anywhere else — behind an assistant message,
///   or opening the conversation — becomes ONE user message, the
///   texts joined by blank lines.
pub fn messages(
    system_prompt: Option<String>,
    continuation: Option<Continuation>,
    prompt: String,
) -> Vec<Message> {
    let mut messages = Vec::new();
    let mut current: Option<super::AssistantMessage> = None;

    if let Some(content) = system_prompt {
        messages.push(Message::System(super::SystemMessage::new(content)));
    }

    let mut items = continuation
        .into_iter()
        .flat_map(|history| history.0)
        .peekable();
    while let Some(item) = items.next() {
        match item {
            ContinuationItem::Prompt(text) => {
                // The whole run of consecutive prompts, because what
                // they become is decided together.
                let mut run = vec![text];
                while matches!(
                    items.peek(),
                    Some(ContinuationItem::Prompt(_))
                ) {
                    let Some(ContinuationItem::Prompt(text)) = items.next()
                    else {
                        unreachable!("peeked a prompt above");
                    };
                    run.push(text);
                }
                // Directly behind a tool response: steered messages,
                // folded into it. Anywhere else: one user message.
                if current.is_none()
                    && matches!(messages.last(), Some(Message::Tool(_)))
                {
                    let Some(Message::Tool(tool)) = messages.last_mut()
                    else {
                        unreachable!("matched a tool message above");
                    };
                    tool.fold_steer(&run);
                } else {
                    if let Some(assistant) = current.take() {
                        messages.push(Message::Assistant(assistant));
                    }
                    messages.push(Message::User(super::UserMessage::new(
                        run.join("\n\n"),
                    )));
                }
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
                | AgenticLoopChunk::User(_) => {}
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

