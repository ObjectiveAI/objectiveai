//! Whose a dependency tool is: one agent's, one agent template's, or
//! everyone's.

use serde::{Deserialize, Serialize};

use super::Agent;

/// Whose a dependency tool is, as its template's `scope` named it:
/// the one agent it is for — a transient tool's always, a service's
/// whose template's `scope` is `instance` — the agent template whose
/// agents it is for, or nobody in particular, every agent that
/// declares it being attached to the one. Externally tagged on the
/// wire, snake case: `{"agent":{"template":…,"index":…}}`,
/// `{"agent_template":{"template":…}}`, or the string `"global"`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Owner {
    /// The one agent, once and for all: a transient tool's, or a
    /// service's whose template's `scope` is `instance`.
    Agent(Agent),
    /// Every agent made from the template: a service's whose
    /// template's `scope` is `template`.
    AgentTemplate {
        /// The agent template, by id.
        template: String,
    },
    /// Every agent that declares the template: a service's whose
    /// template's `scope` is `global`.
    Global,
}
