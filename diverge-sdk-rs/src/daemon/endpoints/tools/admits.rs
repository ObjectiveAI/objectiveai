//! What an admission lets its identity do.

use serde::{Deserialize, Serialize};

/// Which of the two questions a provider asks about a tool the daemon
/// runs the admission answers yes to: listing it, joining it, or both.
/// Snake case on the wire: `"list"`, `"connect"`, `"both"`. An
/// admission that answers neither is no admission, so there is no such
/// value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Admits {
    /// The identity may see the tool when it asks its provider which
    /// tool containers the daemon runs, as the provider protocol's
    /// `authorize_list` asks: it learns the container exists and its
    /// id, and nothing more. No key is minted.
    List,
    /// A connector presenting the admission's key as its
    /// `authorization` may attach to the tool, as the provider
    /// protocol's `authorize_connect` asks. The key is minted and
    /// answered once.
    Connect,
    /// Both of the above: the identity may see the tool, and the key
    /// joins it.
    Both,
}
