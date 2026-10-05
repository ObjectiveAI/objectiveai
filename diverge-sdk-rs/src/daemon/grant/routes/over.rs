//! What is done to routes that exist.

use serde::{Deserialize, Serialize};

/// The actions over routes that exist, which a grant reaches as far as
/// its `within` says. Snake case on the wire: `"delete"`, `"list"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Over {
    /// Take a route up, as
    /// [`tools::routes::delete`](crate::daemon::endpoints::tools::routes::delete)
    /// does.
    Delete,
    /// List them, as
    /// [`tools::routes::list`](crate::daemon::endpoints::tools::routes::list)
    /// does; the list sends what the grant reaches.
    List,
}
