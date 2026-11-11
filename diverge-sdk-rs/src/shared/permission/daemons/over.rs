//! What is done to, and through, remote daemons' records.

use serde::{Deserialize, Serialize};

/// The actions over remote daemons' records, which a grant reaches as
/// far as its `within` says. Snake case on the wire: `"get"`, `"list"`,
/// `"edit"`, `"delete"`, `"connect"`, `"relay"`, `"oci"`.
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
    /// Hold a connected tool on one, as
    /// [`connected_standalone_tools::create`](crate::daemon::endpoints::connected_standalone_tools::create)
    /// does.
    Connect,
    /// Relay any request to one, served there: the first ID of a
    /// request's `relay`, see [`Relay`](crate::shared::relay::Relay).
    Relay,
    /// Ask one for images, as
    /// [`images::oci`](crate::daemon::endpoints::images::oci) does; the
    /// daemon asks only the remote daemons whose records the grant
    /// reaches.
    Oci,
}
