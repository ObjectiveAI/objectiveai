//! One message of the conversation.

use serde::{Deserialize, Serialize};

use super::{AssistantMessage, ToolMessage, UserMessage};

/// One message of the conversation handed to the model.
///
/// Untagged, with each payload carrying its own `role` constant — the
/// same discipline the response chunks use for `type`. serde has no
/// tag of its own to read, so the wire shape is the message itself
/// rather than a wrapper around one, and no two variants can produce
/// the same `role`.
///
/// There is no system role. A system prompt is a
/// [`UserMessage`](super::UserMessage) at the front, or provider
/// configuration — inventing a third message kind for it would mean
/// every provider that does not have one has to decide what to do with
/// it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Message {
    /// From the caller.
    User(UserMessage),
    /// From the model.
    Assistant(AssistantMessage),
    /// From a tool.
    Tool(ToolMessage),
}
