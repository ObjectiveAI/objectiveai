//! One dependency position.

use serde::{Deserialize, Serialize};

/// Where a tool is asked for: the agent whose run asks, by name, and
/// the template of the dependency itself. An agent `a` that declares
/// a dependency of template `t1` asks for `t1` at
/// `{"agent":"a","template":"t1"}`. Only an agent has dependencies —
/// a tool container declares none — so a position is one agent and
/// one template, and no chain.
///
/// The agent is named by name, not by template and index, because a
/// route is the caller's standing instruction: it answers whatever
/// agent of that name runs, now and after the name is given again.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Path {
    /// The agent that asks, by name.
    pub agent: String,
    /// The dependency's template, by id: the only template a tool
    /// routed here may be made from.
    pub template: String,
}
