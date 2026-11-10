//! The `item.started` event: an item beginning.

use serde::Deserialize;

use super::item::Item;

/// A `type: "item.started"` event: a new item, typically in
/// progress. Written for commands, MCP calls, web searches, collab
/// calls and the first plan of a todo list — never for an agent
/// message or a reasoning summary, which arrive whole at
/// `item.completed`. A file change is documented by the source as
/// completed only; the mapping would pass a start through if the
/// core sent one, and whether it does is a live-run fact.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ItemStarted {
    /// Always `item.started`.
    pub r#type: ItemStartedType,
    /// The item, as it begins.
    pub item: Item,
}

/// The `item.started` literal.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize)]
pub enum ItemStartedType {
    /// The only value.
    #[default]
    #[serde(rename = "item.started")]
    ItemStarted,
}
