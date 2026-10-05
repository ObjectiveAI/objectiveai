//! What is done to tool templates that exist.

use serde::{Deserialize, Serialize};

/// The actions over tool templates that exist, which a grant reaches as
/// far as its `within` says. Snake case on the wire: `"get"`,
/// `"delete"`, `"list"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Over {
    /// Get one, as
    /// [`tools::templates::get`](crate::daemon::endpoints::tools::templates::get)
    /// does.
    Get,
    /// Delete one, as
    /// [`tools::templates::delete`](crate::daemon::endpoints::tools::templates::delete)
    /// does.
    Delete,
    /// List them, as
    /// [`tools::templates::list`](crate::daemon::endpoints::tools::templates::list)
    /// does; the list sends what the grant reaches.
    List,
}
