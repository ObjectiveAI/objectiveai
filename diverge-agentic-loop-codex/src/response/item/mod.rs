//! The items: what a turn is made of.
//!
//! An item is `{id, type, ...}` — the source's `ThreadItem`, an id
//! beside a `type`-tagged payload flattened into the same object.
//! That is the one place this crate keeps a tagged enum instead of
//! marker fields: the tag sits INSIDE the flattened item, where a
//! leaf's own marker could not reach it, so [`ItemDetails`] carries
//! the source's own `#[serde(tag = "type")]`. The nine payloads are
//! the pinned version's complete set.
//!
//! Ids are the processor's, `item_<n>`, minted per process; a
//! resumed thread's next process starts counting again. An item's
//! id is the same across its started, updated and completed events.

mod agent_message;
mod collab_tool_call;
mod command_execution;
mod error;
mod file_change;
mod mcp_tool_call;
mod reasoning;
mod todo_list;
mod web_search;

pub use agent_message::*;
pub use collab_tool_call::*;
pub use command_execution::*;
pub use error::*;
pub use file_change::*;
pub use mcp_tool_call::*;
pub use reasoning::*;
pub use todo_list::*;
pub use web_search::*;

use serde::Deserialize;

/// One item: its id, and its payload flattened beside it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Item {
    /// The processor's id for the item, `item_<n>`.
    pub id: String,
    /// The payload, discriminated by its `type`.
    #[serde(flatten)]
    pub details: ItemDetails,
}

/// The item's payload, one of nine.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ItemDetails {
    /// The agent's response — natural language, or a JSON string
    /// when structured output was requested. Completed only, whole.
    AgentMessage(AgentMessage),
    /// The agent's reasoning summary. Completed only, whole.
    Reasoning(Reasoning),
    /// A command the agent ran: started when spawned, completed when
    /// the process exits.
    CommandExecution(CommandExecution),
    /// A set of file changes: completed once the patch succeeds or
    /// fails.
    FileChange(FileChange),
    /// A call to an MCP tool: started when dispatched, completed when
    /// the server reports.
    McpToolCall(McpToolCall),
    /// A call to a collab tool — another agent thread.
    CollabToolCall(CollabToolCall),
    /// A web search: started when kicked off, completed with the
    /// results returned.
    WebSearch(WebSearch),
    /// The agent's running to-do list: started with the first plan,
    /// updated as steps change, completed at the turn's end.
    TodoList(TodoList),
    /// A non-fatal error, surfaced as an item.
    Error(ErrorItem),
}
