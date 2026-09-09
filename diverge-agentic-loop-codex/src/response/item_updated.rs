//! The `item.updated` event: an item changing.

use serde::Deserialize;

use super::item::Item;

/// A `type: "item.updated"` event. At the pin only the todo list is
/// ever updated — each plan revision after the first, under the same
/// item id — and it is completed at the turn's end.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ItemUpdated {
    /// Always `item.updated`.
    pub r#type: ItemUpdatedType,
    /// The item, as it now is.
    pub item: Item,
}

/// The `item.updated` literal.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize)]
pub enum ItemUpdatedType {
    /// The only value.
    #[default]
    #[serde(rename = "item.updated")]
    ItemUpdated,
}
