//! What is done to connected tools that exist.

use serde::{Deserialize, Serialize};

/// The actions over connected tools that exist, which a grant reaches
/// as far as its `within` says. Snake case on the wire: `"get"`,
/// `"list"`, `"edit"`, `"delete"`, `"attach"`, `"detach"`, `"logs"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Over {
    /// Get one, as
    /// [`connected_standalone_tools::get`](crate::daemon::endpoints::connected_standalone_tools::get)
    /// does.
    Get,
    /// List them, as
    /// [`connected_standalone_tools::list`](crate::daemon::endpoints::connected_standalone_tools::list)
    /// does; the list sends what the grant reaches.
    List,
    /// Change one's name, as
    /// [`connected_standalone_tools::edit`](crate::daemon::endpoints::connected_standalone_tools::edit)
    /// does.
    Edit,
    /// Delete one, as
    /// [`connected_standalone_tools::delete`](crate::daemon::endpoints::connected_standalone_tools::delete)
    /// does.
    Delete,
    /// Attach one to an agent, as
    /// [`attach_agent_instance`](crate::daemon::endpoints::connected_standalone_tools::attach_agent_instance)
    /// does, the agent judged by an `edit` grant over agents; or to
    /// every agent of a template, as
    /// [`attach_agent_template`](crate::daemon::endpoints::connected_standalone_tools::attach_agent_template)
    /// does, the template judged by an `attach` grant over agent
    /// templates.
    Attach,
    /// Detach one from an agent, as
    /// [`detach_agent_instance`](crate::daemon::endpoints::connected_standalone_tools::detach_agent_instance)
    /// does, the agent judged by an `edit` grant over agents; or from
    /// every agent of a template, as
    /// [`detach_agent_template`](crate::daemon::endpoints::connected_standalone_tools::detach_agent_template)
    /// does, the template judged by an `attach` grant over agent
    /// templates.
    Detach,
    /// Read one's log, as
    /// [`logs`](crate::daemon::endpoints::logs)
    /// does.
    Logs,
}
