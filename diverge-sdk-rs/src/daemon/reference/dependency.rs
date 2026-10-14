//! One dependency tool, named by its template and whose it is.

use serde::{Deserialize, Serialize};

use super::Owner;

/// One dependency tool of the caller's: its template, by id, and
/// whose it is — the one agent, the agent template, or nobody — see
/// [`Owner`]. A transient tool is its agent's; a service is whose its
/// template's `scope` says. The pair names the tool, which the
/// [`dependency_tools`](crate::daemon::endpoints::dependency_tools)
/// list reports one item for: a template that is none, and an owner
/// the template has no tool under, are each the request's `NotFound`,
/// and so, for an operation on the tool's container, is a tool that
/// does not run. One JSON object: `{"template":…,"owner":…}`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Dependency {
    /// The dependency template, by id: the hash of its canonical
    /// bytes, as the [`dependency_tools`](crate::daemon::endpoints::dependency_tools)
    /// list reports it.
    pub template: String,
    /// Whose the tool is: see [`Owner`].
    pub owner: Owner,
}
