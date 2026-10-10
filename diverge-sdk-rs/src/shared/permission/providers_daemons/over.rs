//! What is done to daemons that exist.

use serde::{Deserialize, Serialize};

/// The actions over daemons that exist, which a grant reaches as far as
/// its `within` says. Snake case on the wire: `"get"`, `"list"`,
/// `"delete"`, `"edit"`.
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
    /// Replace one's mode or its links, as
    /// [`providers::daemons::edit`](crate::daemon::endpoints::providers::daemons::edit)
    /// does.
    Edit,
}
