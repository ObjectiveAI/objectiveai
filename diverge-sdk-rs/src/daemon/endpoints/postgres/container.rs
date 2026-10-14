//! The container holding a connection: an agent, a tool, or a
//! dependency tool.

use serde::{Deserialize, Serialize};

use crate::daemon::reference;

/// Which container holds a database connection — which is to say,
/// which database scope it reaches: an agent or a tool on record,
/// named once and for all — by template and index, or, a connected
/// tool, by daemon and tool — since a name may be absent and the
/// daemon answers the form that always names; or a dependency tool,
/// by the template it runs from and whose it is, the owner in its
/// once-and-for-all form, see [`reference::Dependency`]. On the wire
/// one object with one member, named for the family:
/// `{"agent":{"template":…,"index":…}}`,
/// `{"tool":{"daemon":…,"tool":…}}` or
/// `{"dependency":{"template":…,"owner":…}}`.
///
/// # One scope per owner and part
///
/// The daemon's role and schema for a scope name its OWNER first:
/// `diverge_`, the first twenty hexadecimal characters of the SHA-256
/// of the owner's canonical bytes — the agent or the tool for a
/// container's own scope; the agent, the agent template, or the
/// template itself for a dependency tool's, as its owner says — then
/// the first twenty of the SHA-256 of the scope's part within the
/// owner: `null` for a container's own scope and for a global
/// service's, the dependency's template id as a JSON string for any
/// other dependency tool's. Two dependency tools of one template
/// under one owner reach one scope; under two owners, two. A scope,
/// once made, is dropped by exactly one delete: a tool's own by the
/// tool's, a dependency tool's by its
/// [template's](crate::daemon::endpoints::dependency_tools::delete), and
/// an agent's own by none — an agent's delete and an agent template's
/// drop no scope.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Container {
    /// An agent, as [`reference::Agent`] names one.
    Agent(reference::Agent),
    /// A tool, as [`reference::Tool`] names one.
    Tool(reference::Tool),
    /// A dependency tool, as [`reference::Dependency`] names one.
    Dependency(reference::Dependency),
}
