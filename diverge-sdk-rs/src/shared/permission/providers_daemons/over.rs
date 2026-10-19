//! What is done to daemons that exist.

use serde::{Deserialize, Serialize};

/// The actions over daemons that exist, which a grant reaches as far as
/// its `within` says — and, for the two done AS one of the record's
/// accounts, as far as its `accounts` says. Snake case on the wire:
/// `"get"`, `"list"`, `"delete"`, `"edit"`, `"connect"`, `"relay"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Over {
    /// Get one, as
    /// [`providers::daemons::get`](crate::daemon::endpoints::providers::daemons::get)
    /// does.
    Get,
    /// List them, as
    /// [`providers::daemons::list`](crate::daemon::endpoints::providers::daemons::list)
    /// does; the list sends what the grant reaches.
    List,
    /// Forget one, as
    /// [`providers::daemons::delete`](crate::daemon::endpoints::providers::daemons::delete)
    /// does.
    Delete,
    /// Replace one's links, as
    /// [`providers::daemons::edit`](crate::daemon::endpoints::providers::daemons::edit)
    /// does; and add, rotate and forget its accounts, as
    /// [`providers::daemons::accounts`](crate::daemon::endpoints::providers::daemons::accounts)
    /// does.
    Edit,
    /// Reach a tool of the daemon's as one of the record's accounts,
    /// as
    /// [`standalone_tools::register`](crate::daemon::endpoints::standalone_tools::register)
    /// names one and the connected tool's use opens it. Judged against
    /// the grant's `accounts` as well as its `within`.
    Connect,
    /// Put a request to the daemon as one of the record's accounts —
    /// any request of the wire carrying a `relay` naming the record
    /// and the account. Judged against the grant's `accounts` as well
    /// as its `within`.
    Relay,
}
