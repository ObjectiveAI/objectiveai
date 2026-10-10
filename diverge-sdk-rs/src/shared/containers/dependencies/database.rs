//! Which database a tool gets.

use serde::{Deserialize, Serialize};

/// How the tool's database scope is shared, if it is: by the parent
/// agent, or by the parent agent's template. A tool container reaches
/// its database through the caller, which serves it one scope of its
/// own — a role and a schema, as the daemon makes them — and this says
/// which scope that is. Snake case on the wire: `"per_agent_instance"`,
/// `"per_agent_template"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Database {
    /// One scope per parent agent: the tool deployed for one agent has
    /// a database of its own, and two agents' tools of this dependency
    /// never see each other's.
    PerAgentInstance,
    /// One scope per parent agent template: every tool of this
    /// dependency deployed under any agent made from the same agent
    /// template shares one database, so what one writes the others
    /// read.
    PerAgentTemplate,
}
