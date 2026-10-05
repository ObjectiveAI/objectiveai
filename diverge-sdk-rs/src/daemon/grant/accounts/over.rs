//! What is done to accounts that exist.

use serde::{Deserialize, Serialize};

/// The actions over accounts that exist, which a grant reaches as far
/// as its `within` says. Snake case on the wire: `"get"`, `"list"`,
/// `"delete"`, `"edit"`, `"assign"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Over {
    /// Get one, as
    /// [`accounts::get`](crate::daemon::endpoints::accounts::get) does.
    Get,
    /// List them, as
    /// [`accounts::list`](crate::daemon::endpoints::accounts::list)
    /// does; the list sends what the grant reaches.
    List,
    /// Delete one, as
    /// [`accounts::delete`](crate::daemon::endpoints::accounts::delete)
    /// does.
    Delete,
    /// Change one, as
    /// [`accounts::edit`](crate::daemon::endpoints::accounts::edit)
    /// does.
    Edit,
    /// Name one as a container's `account`, at an
    /// [agent's](crate::daemon::endpoints::agents::create) or a
    /// [tool's](crate::daemon::endpoints::tools::create) create or
    /// edit.
    Assign,
}
