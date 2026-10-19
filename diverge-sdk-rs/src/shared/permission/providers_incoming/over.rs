//! What is done to incoming credentials that exist.

use serde::{Deserialize, Serialize};

/// The actions over incoming credentials that exist, which a grant
/// reaches as far as its `within` says. Snake case on the wire:
/// `"get"`, `"list"`, `"delete"`, `"edit"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Over {
    /// Get one, as
    /// [`providers::incoming::get`](crate::daemon::endpoints::providers::incoming::get)
    /// does.
    Get,
    /// List them, as
    /// [`providers::incoming::list`](crate::daemon::endpoints::providers::incoming::list)
    /// does; the list sends what the grant reaches.
    List,
    /// Take one out, as
    /// [`providers::incoming::delete`](crate::daemon::endpoints::providers::incoming::delete)
    /// does.
    Delete,
    /// Replace one, as
    /// [`providers::incoming::edit`](crate::daemon::endpoints::providers::incoming::edit)
    /// does.
    Edit,
}
