//! Style directions.

use serde::{Deserialize, Serialize};

/// How the agent writes, as direction lines.
///
/// Eliza composes two lists from three: chat style is
/// [`all`](Self::all) then [`chat`](Self::chat), post style is
/// [`all`](Self::all) then [`post`](Self::post). The loop's one room
/// is a chat room, so [`post`](Self::post) never renders here; it is
/// kept because the character is one document and a caller may
/// carry it whole.
#[derive(
    Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct Style {
    /// Directions for everything the agent writes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub all: Vec<String>,
    /// Directions for conversation, after [`all`](Self::all).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chat: Vec<String>,
    /// Directions for posts, after [`all`](Self::all). Never rendered
    /// in the loop's room.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub post: Vec<String>,
}

impl Style {
    /// Whether every list is empty — the condition for leaving the
    /// field off the wire.
    pub fn is_empty(&self) -> bool {
        self.all.is_empty() && self.chat.is_empty() && self.post.is_empty()
    }
}
