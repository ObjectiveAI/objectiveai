//! The kinds of item a command log holds, by name.

use serde::{Deserialize, Serialize};

use super::Item;

/// One kind of [`Item`], as its `type` names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ItemType {
    /// [`Item::Command`].
    Command,
}

impl ItemType {
    /// The kind of `item`.
    pub fn of(item: &Item) -> ItemType {
        match item {
            Item::Command(_) => ItemType::Command,
        }
    }

    /// The kind's name, as `type` carries it on the wire.
    pub fn name(self) -> &'static str {
        match self {
            ItemType::Command => "command",
        }
    }
}
