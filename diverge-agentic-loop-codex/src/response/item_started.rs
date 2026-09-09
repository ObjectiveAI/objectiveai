//! The `item.started` event: an item beginning.

use serde::Deserialize;

use super::item::Item;

/// A `type: "item.started"` event: a new item, typically in
/// progress. Written for commands, MCP calls, web searches, file
/// changes, collab calls and the first plan of a todo list — never
/// for an agent message or a reasoning summary, which arrive whole
/// at `item.completed`.
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
