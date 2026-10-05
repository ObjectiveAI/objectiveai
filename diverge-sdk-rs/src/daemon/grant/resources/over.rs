//! What is done to resources that exist.

use serde::{Deserialize, Serialize};

/// The actions over resources that exist, which a grant reaches as far
/// as its `within` says. Snake case on the wire: `"list"`, `"delete"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Over {
    /// List them, as
    /// [`resources::list`](crate::daemon::endpoints::resources::list)
    /// does; the list sends what the grant reaches.
    List,
    /// Delete one, as
    /// [`resources::delete`](crate::daemon::endpoints::resources::delete)
    /// does.
    Delete,
}
