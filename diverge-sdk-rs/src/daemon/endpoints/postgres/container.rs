//! The container holding a connection: an agent, a tool, or a
//! dependency tool.

use serde::{Deserialize, Serialize};

use super::Dependency;
use crate::daemon::reference;

/// Which container holds a database connection — which is to say,
/// which database scope it reaches: an agent or a tool on record,
/// named once and for all — by template and index, or, a connected
/// tool, by provider and id — since a name may be absent and the
/// daemon answers the form that always names; or a dependency tool's
/// scope, which is named by what the dependency's template says it
/// shares with, see [`Dependency`]. On the wire one object with one
/// member, named for the family: `{"agent":{"template":…,"index":…}}`,
/// `{"tool":{"provider":…,"id":…}}` or
/// `{"dependency":{"parent":…,"name":…}}`. The daemon's role and
/// schema for the scope is a hash of exactly this JSON.
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
