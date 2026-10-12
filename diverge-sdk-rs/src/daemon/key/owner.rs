//! Whose a service is: one agent's, one agent template's, or
//! everyone's.

use serde::{Deserialize, Serialize};

use super::Agent;

/// The scope key a service was made under, as its template's `scope`
/// named it: the one agent it is for, the agent template whose
/// agents it is for, or nobody in particular, every agent that
/// declares it being attached to the one. Externally tagged on the
/// wire, snake case: `{"agent":{"template":…,"index":…}}`,
/// `{"agent_template":{"template":…}}`, or the string `"global"`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Owner {
    /// The one agent, once and for all: the template's `scope` is
    /// `per_agent_instance`.
    Agent(Agent),
    /// Every agent made from the template: the template's `scope` is
    /// `per_agent_template`.
    AgentTemplate {
        /// The agent template, by id.
        template: String,
    },
    /// Every agent that declares it: the template's `scope` is
    /// `global`.
    Global,
}
