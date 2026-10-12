//! One tool, once and for all.

use serde::{Deserialize, Serialize};

use super::{Agent, Origin, Owner};

/// One tool of the caller's, once and for all: a record, by its fixed
/// [`Origin`] and its index, with its name beside when it has one; a
/// transient dependency, by the agent it was deployed for and the
/// template it was deployed from; or a service, by the template it
/// was made from and whose it is. Externally tagged on the wire,
/// snake case: `{"record":{…}}`, `{"dependency":{…}}` or
/// `{"service":{…}}`.
///
/// A transient dependency tool is no record: it is deployed when its
/// agent's container starts, from the template the agent's program
/// declared, and ends with the agent's run. What names it once and
/// for all is its agent — itself once and for all, by template and
/// index — and its template's id, unique among the agent's
/// dependencies. A service is a record of the daemon's own making,
/// made when the first agent that declares its template registers
/// and kept until the last attached agent leaves it: what names it
/// once and for all is its template's id and its [`Owner`], the
/// scope key it was made under.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tool {
    /// A tool on record: made by a create, or joined by a connect.
    Record {
        /// What it was made with: see [`Origin`].
        origin: Origin,
        /// Its number among all tools of the caller's ever made with
        /// that origin, as its list item carries it.
        index: u64,
        /// Its name, as its create or its connect gave it, if it gave
        /// one. Absent when it has none.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    /// A transient dependency tool: deployed for an agent's run, from
    /// a template the agent's program declared.
    Dependency {
        /// The agent it was deployed for: see [`Agent`].
        agent: Agent,
        /// The dependency tool template it was deployed from, by id —
        /// the hash of the template's canonical bytes — unique among
        /// that agent's dependencies.
        template: String,
    },
    /// A service: made from a template an agent's program declared,
    /// under the scope key the template's `scope` names, and kept for
    /// every agent attached to it.
    Service {
        /// The dependency tool template it was made from, by id —
        /// the hash of the template's canonical bytes.
        template: String,
        /// Whose it is: see [`Owner`].
        owner: Owner,
    },
}
