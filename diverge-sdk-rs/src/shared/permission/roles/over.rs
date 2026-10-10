//! What is done to roles that exist.

use serde::{Deserialize, Serialize};

/// The actions over roles that exist, which a grant reaches as far as
/// its `within` says. Snake case on the wire: `"get"`, `"list"`,
/// `"delete"`, `"edit"`, `"grant"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Over {
    /// Get one, as [`roles::get`](crate::daemon::endpoints::roles::get)
    /// does.
    Get,
    /// List them, as
    /// [`roles::list`](crate::daemon::endpoints::roles::list) does; the
    /// list sends what the grant reaches.
    List,
    /// Delete one, as
    /// [`roles::delete`](crate::daemon::endpoints::roles::delete) does.
    Delete,
    /// Change one, as
    /// [`roles::edit`](crate::daemon::endpoints::roles::edit) does.
    Edit,
    /// Name one in an account's `roles`, at the account's
    /// [create](crate::daemon::endpoints::accounts::create) or
    /// [edit](crate::daemon::endpoints::accounts::edit).
    Grant,
}
