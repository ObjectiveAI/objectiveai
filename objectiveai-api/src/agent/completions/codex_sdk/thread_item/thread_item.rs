use serde::{Deserialize, Serialize};

/// A typed item carried inside `item.started` / `item.updated` /
/// `item.completed` events. The wire shape is `{"type": <discriminator>,
/// ...fields}` — each leaf struct carries its own `r#type` field, so the
/// parent enum is `untagged` and serde dispatches by trying each variant
/// in declaration order.
///
/// The [`Self::Unknown`] variant mirrors `UnknownThreadItem` in
/// `types.py:197-204`: any item whose `type` we don't recognise still
/// parses, preserving the discriminator and `id` for forward compatibility.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ThreadItem {
    Known(KnownThreadItem),
    Unknown(UnknownThreadItem),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum KnownThreadItem {
    AgentMessage(super::AgentMessageItem),
    Reasoning(super::ReasoningItem),
    CommandExecution(super::CommandExecutionItem),
    FileChange(super::FileChangeItem),
    McpToolCall(super::McpToolCallItem),
    WebSearch(super::WebSearchItem),
    TodoList(super::TodoListItem),
    Error(super::ErrorItem),
}

/// Forward-compat fallback for items with a `type` we don't recognise.
/// Mirrors `UnknownThreadItem` in `types.py:197-204`. Only `id` and the
/// discriminator are preserved — extra payload fields are discarded since
/// the consumer can't act on them anyway.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnknownThreadItem {
    pub id: String,
    pub r#type: String,
}
