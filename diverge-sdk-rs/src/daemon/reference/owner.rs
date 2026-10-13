//! Whose a dependency tool is: one agent's, one agent template's, or
//! everyone's.

use serde::{Deserialize, Serialize};

use super::Agent;

/// Whose a dependency tool is, as its template's `scope` names it:
/// the one agent it is for — a transient tool's always, a service's
/// whose template's `scope` is `instance` — the agent template whose
/// agents it is for, or nobody in particular, every agent that
/// declares it being attached to the one. The agent is named either
/// way, by name or once and for all: see [`Agent`]. Externally
/// tagged on the wire, snake case: `{"agent":{"name":…}}` or
/// `{"agent":{"template":…,"index":…}}`,
/// `{"agent_template":{"template":…}}`, or the string `"global"`.
/// The daemon answers the once-and-for-all form, as
/// [`key::Owner`](crate::daemon::key::Owner).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Owner {
    /// The one agent: a transient tool's, or a service's whose
    /// template's `scope` is `instance`.
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
