//! The container holding a connection, an agent or a tool.

use serde::{Deserialize, Serialize};

use crate::daemon::reference;

/// Which container holds a database connection: an agent or a tool,
/// named once and for all — by template and index, or, a connected
/// tool, by provider and id — since a name may be absent and the daemon
/// answers the form that always names. On the wire one object with one
/// member, named for the family: `{"agent":{"template":…,"index":…}}`
/// or `{"tool":{"provider":…,"id":…}}`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Container {
    /// An agent, as [`reference::Agent`] names one.
    Agent(reference::Agent),
    /// A tool, as [`reference::Tool`] names one.
    Tool(reference::Tool),
}
