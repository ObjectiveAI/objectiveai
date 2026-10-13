//! One dependency tool, named by its template and whose it is.

use serde::{Deserialize, Serialize};

use super::Owner;

/// One dependency tool of the caller's: the template it runs from, by
/// id, and whose it is — the one agent, the agent template, or
/// everyone — see [`Owner`]. A transient tool is its agent's; a
/// service is whose its template's `scope` says. The pair names the
/// tool while it runs and names nothing else: a template that is
/// none, an owner the template has no tool for, and a tool that does
/// not run are each the request's `NotFound`. One JSON object:
/// `{"template":…,"owner":…}`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Dependency {
    /// The dependency template, by id: the hash of its canonical
    /// bytes, as the [`dependency_tools`](crate::daemon::endpoints::dependency_tools)
    /// list reports it.
    pub template: String,
    /// Whose the tool is: see [`Owner`].
    pub owner: Owner,
}
