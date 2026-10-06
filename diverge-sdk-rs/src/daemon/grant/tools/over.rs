//! What is done to tools that exist.

use serde::{Deserialize, Serialize};

/// The actions over tools that exist, which a grant reaches as far as
/// its `within` says. Snake case on the wire: `"get"`, `"edit"`,
/// `"attach"`, `"detach"`, `"delete"`, `"list"`, `"download"`,
/// `"upload"`, `"transfer"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Over {
    /// Get one, as [`tools::get`](crate::daemon::endpoints::tools::get)
    /// does.
    Get,
    /// Change one, as
    /// [`tools::edit`](crate::daemon::endpoints::tools::edit) does.
    Edit,
    /// Attach one to an agent, as
    /// [`tools::attach`](crate::daemon::endpoints::tools::attach) does;
    /// the agent is judged by an `edit` grant over agents.
    Attach,
    /// Detach one from an agent, as
    /// [`tools::detach`](crate::daemon::endpoints::tools::detach) does;
    /// the agent is judged by an `edit` grant over agents.
    Detach,
    /// Delete one, as
    /// [`tools::delete`](crate::daemon::endpoints::tools::delete) does.
    Delete,
    /// List them, as
    /// [`tools::list`](crate::daemon::endpoints::tools::list) does; the
    /// list sends what the grant reaches.
    List,
    /// Send the client files out of one, as
    /// [`tools::download`](crate::daemon::endpoints::tools::download)
    /// does.
    Download,
    /// Put files into one, as
    /// [`tools::upload`](crate::daemon::endpoints::tools::upload) does,
    /// and as a transfer landing in one does.
    Upload,
    /// Copy files out of one, as
    /// [`tools::transfer`](crate::daemon::endpoints::tools::transfer)
    /// does; where they land is judged by its own grant.
    Transfer,
}
