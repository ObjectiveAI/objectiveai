//! What is done to, and through, providers' records.

use serde::{Deserialize, Serialize};

/// The actions over providers' records, which a grant reaches as far
/// as its `within` says: the providers whose records the `within`
/// reaches, by their tags. Snake case on the wire: `"get"`, `"list"`,
/// `"oci"`, `"edit"`, `"delete"`, `"enable"`, `"disable"`. What is done
/// through them to the daemons connected to them is the
/// [`providers_daemons`](crate::shared::permission::providers_daemons)
/// grant's.
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
    /// Ask them for images, as
    /// [`images::oci`](crate::daemon::endpoints::images::oci) does; the
    /// daemon asks only the providers the grant reaches.
    Oci,
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
