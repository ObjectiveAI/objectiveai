//! The container holding a connection: an agent, a tool, or a
//! dependency tool.

use serde::{Deserialize, Serialize};

use super::Dependency;
use crate::daemon::reference;

/// Which container holds a database connection — which is to say,
/// which database scope it reaches: an agent or a tool on record,
/// named once and for all — by template and index, or, a connected
/// tool, by daemon and tool — since a name may be absent and the
/// daemon answers the form that always names; or a dependency tool's
/// scope, which is named by what the dependency's template says it
/// shares with, see [`Dependency`]. On the wire one object with one
/// member, named for the family: `{"agent":{"template":…,"index":…}}`,
/// `{"tool":{"daemon":…,"tool":…}}` or
/// `{"dependency":{"parent":…,"template":…}}`. The daemon's role
/// and schema for the scope name its OWNER first: `diverge_`, the
/// first twenty hexadecimal characters of the SHA-256 of the owner's
/// canonical bytes — the agent or the tool the scope is a container's
/// own, or the dependency's parent — then the first twenty of the
/// SHA-256 of the scope's part within the owner, `null` for a
/// container's own scope and the dependency's template id as a JSON
/// string for a dependency's; so that everything an agent owns, its
/// own scope and its per-instance dependencies' scopes, shares its
/// prefix and goes with it when it is deleted, and what an agent
/// template owns does not.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Container {
    /// An agent, as [`reference::Agent`] names one.
    Agent(reference::Agent),
    /// A tool, as [`reference::Tool`] names one.
    Tool(reference::Tool),
    /// A dependency tool's scope: see [`Dependency`].
    Dependency(Dependency),
}
