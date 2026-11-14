//! What is done to, and through, remote daemons' records.

use serde::{Deserialize, Serialize};

/// The actions over remote daemons' records, which a grant reaches as
/// far as its `within` says. Snake case on the wire: `"get"`, `"list"`,
/// `"edit"`, `"delete"`, `"relay"`. The first four are over the record
/// itself; `relay` is the one permission to USE the daemon the record
/// reaches, whatever the use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Over {
    /// Read one, as
    /// [`daemons::get`](crate::daemon::endpoints::daemons::get) does.
    Get,
    /// List them, as
    /// [`daemons::list`](crate::daemon::endpoints::daemons::list) does;
    /// the list sends what the grant reaches.
    List,
    /// Change one's description, as
    /// [`daemons::edit`](crate::daemon::endpoints::daemons::edit) does.
    Edit,
    /// Delete one, as
    /// [`daemons::delete`](crate::daemon::endpoints::daemons::delete)
    /// does.
    Delete,
    /// Use the daemon the record reaches, as the account the record
    /// names. One action for every use, since each of them is this
    /// daemon asking that one for something as that account:
    ///
    /// 1. Relay any request to it, served there: the first ID of a
    ///    request's `relay`, see [`Relay`](crate::shared::relay::Relay).
    /// 2. Hold a connected tool on it, as
    ///    [`connected_standalone_tools::create`](crate::daemon::endpoints::connected_standalone_tools::create)
    ///    does.
    /// 3. Name a volume on it in a container's FUSE mounts, at the
    ///    container's create or edit.
    /// 4. Ask it for images, as
    ///    [`images::read`](crate::daemon::endpoints::images::read)
    ///    does: the daemon asks only the remote daemons whose records
    ///    the grant reaches.
    ///
    /// What the daemon reached then allows that account is that
    /// daemon's to judge, by its own grants.
    Relay,
}
