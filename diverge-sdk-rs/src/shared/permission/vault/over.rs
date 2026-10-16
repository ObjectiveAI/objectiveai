//! What is done to vault keys that exist.

use serde::{Deserialize, Serialize};

/// The actions over vault keys that exist, which a grant reaches as
/// far as its `within` says. Snake case on the wire: `"get"`,
/// `"list"`, `"set"`, `"delete"`, `"use"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Over {
    /// Get one, with its value, as
    /// [`vault::get`](crate::daemon::endpoints::vault::get) does.
    Get,
    /// List them, as
    /// [`vault::list`](crate::daemon::endpoints::vault::list) does;
    /// the list sends what the grant reaches, and never a value.
    List,
    /// Replace one's value, as
    /// [`vault::set`](crate::daemon::endpoints::vault::set) does over
    /// a key the vault holds.
    Set,
    /// Delete one, as
    /// [`vault::delete`](crate::daemon::endpoints::vault::delete)
    /// does.
    Delete,
    /// Bind one to a secret of an agent's or a tool's, in the
    /// `secrets` of its [create](crate::daemon::create::Inner::secrets)
    /// or edit. A key the vault does not hold yet is reached only by
    /// a grant whose `within` is `"any"`.
    Use,
}
