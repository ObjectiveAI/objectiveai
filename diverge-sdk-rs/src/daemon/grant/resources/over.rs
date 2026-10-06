//! What is done to resources that exist.

use serde::{Deserialize, Serialize};

/// The actions over resources that exist, which a grant reaches as far
/// as its `within` says. Snake case on the wire: `"list"`, `"delete"`,
/// `"download"`, `"transfer"`, `"filetree"`.
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
    /// Send the client one, or a part of one, as
    /// [`resources::download`](crate::daemon::endpoints::resources::download)
    /// does.
    Download,
    /// Copy one, or a part of one, elsewhere, as
    /// [`resources::transfer`](crate::daemon::endpoints::resources::transfer)
    /// does; where it lands is judged by its own grant.
    Transfer,
    /// See one's tree, as
    /// [`resources::filetree`](crate::daemon::endpoints::resources::filetree)
    /// does.
    Filetree,
}
