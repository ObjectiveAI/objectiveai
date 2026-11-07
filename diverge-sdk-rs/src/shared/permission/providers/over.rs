//! What is done to, and through, providers' records.

use serde::{Deserialize, Serialize};

/// The actions over providers' records, which a grant reaches as far
/// as its `within` says: the providers whose records the `within`
/// reaches, by their tags. Snake case on the wire: `"get"`, `"list"`,
/// `"discover"`, `"listen"`, `"message"`, `"edit"`, `"delete"`,
/// `"enable"`, `"disable"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Over {
    /// Read one, as
    /// [`providers::get`](crate::daemon::endpoints::providers::get) does.
    Get,
    /// List them, as
    /// [`providers::list`](crate::daemon::endpoints::providers::list)
    /// does; the list sends what the grant reaches.
    List,
    /// Discover the identities their discover listings hold, as
    /// [`providers::discover`](crate::daemon::endpoints::providers::discover)
    /// does; the listing holds only the routes through the providers
    /// the grant reaches.
    Discover,
    /// Hear the messages sent to the daemon through them, as
    /// [`daemons::listen`](crate::daemon::endpoints::daemons::listen)
    /// does; the daemon sends only those that came through the
    /// providers the grant reaches.
    Listen,
    /// Send messages through them, as `daemons::message` does.
    Message,
    /// Change one, as
    /// [`providers::edit`](crate::daemon::endpoints::providers::edit) does.
    Edit,
    /// Delete one, as
    /// [`providers::delete`](crate::daemon::endpoints::providers::delete)
    /// does.
    Delete,
    /// Enable one, as
    /// [`providers::enable`](crate::daemon::endpoints::providers::enable)
    /// does.
    Enable,
    /// Disable one, its runs taken down, as
    /// [`providers::disable`](crate::daemon::endpoints::providers::disable)
    /// does.
    Disable,
}
