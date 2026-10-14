//! One tool, once and for all.

use serde::{Deserialize, Serialize};

use super::{Origin, Owner};

/// One tool of the caller's, once and for all: a record, by its fixed
/// [`Origin`] and its index, with its name beside when it has one; or
/// a dependency tool, by the template it runs from and whose it is.
/// Externally tagged on the wire, snake case: `{"record":{…}}` or
/// `{"dependency":{…}}`.
///
/// A dependency tool is no record of the tools family: it runs from a
/// [dependency template](crate::daemon::endpoints::dependency_tools) an
/// agent's program declared — a transient one for the agent's run,
/// a service for as long as any agent is attached to it. What names
/// it once and for all is its template's id and its [`Owner`]: the
/// agent, itself once and for all, for a transient tool and for a
/// service whose template's `scope` is `instance`; the agent template
/// or everyone for the other two scopes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
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
    /// A dependency tool: running from a template an agent's program
    /// declared, transient or a service.
    Dependency {
        /// The dependency template it runs from, by id — the hash of
        /// the template's canonical bytes.
        template: String,
        /// Whose it is: see [`Owner`].
        owner: Owner,
    },
}
