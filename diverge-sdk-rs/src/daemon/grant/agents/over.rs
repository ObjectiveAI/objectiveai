//! What is done to agents that exist.

use serde::{Deserialize, Serialize};

/// The actions over agents that exist, which a grant reaches as far as
/// its `within` says. Snake case on the wire: `"get"`, `"delete"`,
/// `"edit"`, `"message"`, `"logs"`, `"list"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Over {
    /// Get one, as
    /// [`agents::get`](crate::daemon::endpoints::agents::get) does.
    Get,
    /// Delete one, as
    /// [`agents::delete`](crate::daemon::endpoints::agents::delete)
    /// does.
    Delete,
    /// Change one, as
    /// [`agents::edit`](crate::daemon::endpoints::agents::edit) does.
    Edit,
    /// Send one a message, as
    /// [`agents::message`](crate::daemon::endpoints::agents::message)
    /// does.
    Message,
    /// Read one's log, as
    /// [`agents::logs`](crate::daemon::endpoints::agents::logs) does.
    Logs,
    /// List them, as
    /// [`agents::list`](crate::daemon::endpoints::agents::list) does;
    /// the list sends what the grant reaches.
    List,
}
