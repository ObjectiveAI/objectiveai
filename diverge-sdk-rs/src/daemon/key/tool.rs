//! One tool, once and for all.

use serde::{Deserialize, Serialize};

use super::{Agent, Origin};

/// One tool of the caller's, once and for all: a record, by its fixed
/// [`Origin`] and its index, with its name beside when it has one; or
/// a dependency, by the agent it was deployed for and the name its
/// template declared. Externally tagged on the wire, snake case:
/// `{"record":{…}}` or `{"dependency":{…}}`.
///
/// A dependency tool is no record: it is deployed when its agent's
/// container starts, from the template the agent's program declared,
/// and ends with the agent. What names it once and for all is its
/// agent — itself once and for all, by template and index — and the
/// declared name, unique among the agent's dependencies.
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
    /// A dependency tool: deployed for an agent, from the template the
    /// agent's program declared under a name.
    Dependency {
        /// The agent it was deployed for: see [`Agent`].
        agent: Agent,
        /// The name the agent's program declared it under, unique
        /// among that agent's dependencies.
        name: String,
    },
}
