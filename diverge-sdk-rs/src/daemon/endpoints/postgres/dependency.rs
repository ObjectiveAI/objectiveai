//! A dependency tool's database scope: whose it is.

use serde::{Deserialize, Serialize};

use crate::daemon::reference;

/// The scope a dependency tool reaches, named by whose it is and by
/// the dependency's template: a transient tool's is its one agent's;
/// a service's is shared as far as its template's `scope` says — one
/// per parent agent, one per parent agent template, or one for the
/// template. Two dependencies of one template under one parent reach
/// one scope; under two parents, one scope when the parents are the
/// same template and the template says so, or when the template's
/// scope is global, else two.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Dependency {
    /// Whose the scope is: see [`Parent`].
    pub parent: Parent,
    /// The dependency tool template, by id: the hash of its canonical
    /// bytes. The same template deployed again for the same parent,
    /// on whatever provider, reaches the same scope.
    pub template: String,
}

/// What a dependency tool's scope is shared with: the one agent it
/// was deployed for, every agent made from the same template, or every
/// agent there is. On the wire one object with one member, snake
/// case, or the one word: `{"agent":{"template":…,"index":…}}`,
/// `{"agent_template":{"template":…}}` or `"global"`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Parent {
    /// The one agent, once and for all: a transient tool's, or a
    /// service's whose template's `scope` is `instance`.
    Agent(reference::Agent),
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
