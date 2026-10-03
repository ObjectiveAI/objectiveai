//! One dependency position, from the top.

use serde::{Deserialize, Serialize};

/// Where in a chain of dependencies a tool is asked for: the agent
/// whose run began the chain, by name, and the template ids down the
/// chain, the last being the template of the dependency itself. An
/// agent `a` whose tool of template `t1` declares a dependency of
/// template `t2` asks for `t2` at
/// `{"agent":"a","templates":["t1","t2"]}`; `a`'s own dependency of
/// template `t1` is asked for at `{"agent":"a","templates":["t1"]}`.
///
/// The agent is named by name, not by template and index, because a
/// route is the caller's standing instruction: it answers whatever
/// agent of that name runs, now and after the name is given again.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Path {
    /// The agent that began the chain, by name.
    pub agent: String,
    /// The template ids down the chain, at least one; the last is the
    /// dependency's template, and the only template a tool routed
    /// here may be made from.
    pub templates: Vec<String>,
}
