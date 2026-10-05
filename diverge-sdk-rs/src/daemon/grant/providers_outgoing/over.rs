//! What is done to outgoing providers that exist.

use serde::{Deserialize, Serialize};

/// The actions over outgoing providers that exist, which a grant
/// reaches as far as its `within` says. Snake case on the wire:
/// `"get"`, `"list"`, `"delete"`, `"edit"`, `"list_for"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Over {
    /// Get one, as
    /// [`providers::outgoing::get`](crate::daemon::endpoints::providers::outgoing::get)
    /// does.
    Get,
    /// List them, as
    /// [`providers::outgoing::list`](crate::daemon::endpoints::providers::outgoing::list)
    /// does; the list sends what the grant reaches.
    List,
    /// Forget one, as
    /// [`providers::outgoing::delete`](crate::daemon::endpoints::providers::outgoing::delete)
    /// does.
    Delete,
    /// Replace one's mode, as
    /// [`providers::outgoing::edit`](crate::daemon::endpoints::providers::outgoing::edit)
    /// does.
    Edit,
    /// Ask one which tool containers a tenant runs, as
    /// [`tools::list_for`](crate::daemon::endpoints::tools::list_for)
    /// does.
    ListFor,
}
