//! A dependency tool's database scope: whose it is.

use serde::{Deserialize, Serialize};

use crate::daemon::reference;

/// The scope a dependency tool reaches, named by what its template's
/// `database` says it is shared with and by the dependency's declared
/// name: one scope per parent agent, or one per parent agent template.
/// Two dependencies of one name under one parent reach one scope;
/// under two parents, one scope when the parents are the same
/// template and the template says so, else two.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Dependency {
    /// Whose the scope is: see [`Parent`].
    pub parent: Parent,
    /// The name the agent's program declared the dependency under.
    pub name: String,
}

/// What a dependency tool's scope is shared with: the one agent it
/// was deployed for, or every agent made from the same template. On
/// the wire one object with one member, snake case:
/// `{"agent":{"template":…,"index":…}}` or
/// `{"agent_template":{"template":…}}`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Parent {
    /// The one agent, once and for all: the template's `database` is
    /// `per_agent_instance`.
    Agent(reference::Agent),
    /// Every agent made from the template: the template's `database`
    /// is `per_agent_template`.
    AgentTemplate {
        /// The agent template, by id.
        template: String,
    },
}
