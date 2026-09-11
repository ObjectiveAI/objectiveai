//! The `item.completed` event: an item's terminal state.

use serde::Deserialize;

use super::item::Item;

/// A `type: "item.completed"` event: the item has reached a terminal
/// state, success or failure. Agent messages and reasoning summaries
/// appear only here, whole; a started item completes here with its
/// outcome; and non-fatal news — a warning, a config warning, a
/// deprecation, a model reroute — arrives here as an
/// [`ErrorItem`](super::item::ErrorItem) under a fresh id. At the
/// turn's end every started item still open is completed here too.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ItemCompleted {
    /// Always `item.completed`.
    pub r#type: ItemCompletedType,
    /// The item, finished.
    pub item: Item,
}

/// The `item.completed` literal.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Deserialize)]
pub enum ItemCompletedType {
    /// The only value.
    #[default]
    #[serde(rename = "item.completed")]
    ItemCompleted,
}
