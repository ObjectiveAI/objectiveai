//! What is done to tools that exist.

use serde::{Deserialize, Serialize};

/// The actions over tools that exist, which a grant reaches as far as
/// its `within` says. Snake case on the wire: `"get"`, `"edit"`,
/// `"attach"`, `"detach"`, `"delete"`, `"list"`, `"download"`,
/// `"upload"`, `"transfer"`, `"filetree"`, `"connect"`, `"logs"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Over {
    /// Get one, as [`tools::get`](crate::daemon::endpoints::standalone_tools::get)
    /// does.
    Get,
    /// Change one, as
    /// [`tools::edit`](crate::daemon::endpoints::standalone_tools::edit) does.
    Edit,
    /// Attach one to an agent, as
    /// [`attach_agent_instance`](crate::daemon::endpoints::standalone_tools::attach_agent_instance)
    /// does, the agent judged by an `edit` grant over agents; or to
    /// every agent of a template, as
    /// [`attach_agent_template`](crate::daemon::endpoints::standalone_tools::attach_agent_template)
    /// does, the template judged by an `attach` grant over agent
    /// templates.
    Attach,
    /// Detach one from an agent, as
    /// [`detach_agent_instance`](crate::daemon::endpoints::standalone_tools::detach_agent_instance)
    /// does, the agent judged by an `edit` grant over agents; or from
    /// every agent of a template, as
    /// [`detach_agent_template`](crate::daemon::endpoints::standalone_tools::detach_agent_template)
    /// does, the template judged by an `attach` grant over agent
    /// templates.
    Detach,
    /// Delete one, as
    /// [`tools::delete`](crate::daemon::endpoints::standalone_tools::delete) does.
    Delete,
    /// List them, as
    /// [`tools::list`](crate::daemon::endpoints::standalone_tools::list) does; the
    /// list sends what the grant reaches.
    List,
    /// Send the client files out of one, as
    /// [`tools::download`](crate::daemon::endpoints::standalone_tools::download)
    /// does.
    Download,
    /// Put files into one, as
    /// [`tools::upload`](crate::daemon::endpoints::standalone_tools::upload) does,
    /// and as a transfer landing in one does.
    Upload,
    /// Copy files out of one, as
    /// [`tools::transfer`](crate::daemon::endpoints::standalone_tools::transfer)
    /// does; where they land is judged by its own grant.
    Transfer,
    /// Watch one's container whole, as
    /// [`tools::filetree`](crate::daemon::endpoints::standalone_tools::filetree)
    /// does.
    Filetree,
    /// Serve one to another daemon, as
    /// [`tools::connect`](crate::daemon::endpoints::standalone_tools::connect) does.
    Connect,
    /// Read one's log, as
    /// [`tools::logs`](crate::daemon::endpoints::standalone_tools::logs)
    /// does.
    Logs,
}
