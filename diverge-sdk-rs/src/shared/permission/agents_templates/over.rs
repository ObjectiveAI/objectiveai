//! What is done to agent templates that exist.

use serde::{Deserialize, Serialize};

/// The actions over agent templates that exist, which a grant reaches
/// as far as its `within` says. Snake case on the wire: `"get"`,
/// `"attach"`, `"delete"`, `"list"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Over {
    /// Get one, as
    /// [`agents::templates::get`](crate::daemon::endpoints::agents::templates::get)
    /// does.
    Get,
    /// Attach a tool to every agent of one, or detach one, as
    /// [`attach_agent_template`](crate::daemon::endpoints::standalone_tools::attach_agent_template)
    /// and
    /// [`detach_agent_template`](crate::daemon::endpoints::standalone_tools::detach_agent_template)
    /// do; the tool is judged by an `attach` or a `detach` grant over
    /// tools.
    Attach,
    /// Delete one, as
    /// [`agents::templates::delete`](crate::daemon::endpoints::agents::templates::delete)
    /// does.
    Delete,
    /// List them, as
    /// [`agents::templates::list`](crate::daemon::endpoints::agents::templates::list)
    /// does; the list sends what the grant reaches.
    List,
}
