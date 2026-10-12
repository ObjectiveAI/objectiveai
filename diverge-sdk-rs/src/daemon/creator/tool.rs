//! A tool as a creator.

use serde::{Deserialize, Serialize};

use super::Agent;
use crate::daemon::key;

/// A tool of the client's that made something: one the client made
/// from a template, named once and for all by its template and its
/// index, and by its name as it was called; a transient dependency
/// tool, named once and for all by the agent it was deployed for and
/// the template it was deployed from; or a service, named once and
/// for all by its template and whose it is. A connected tool makes
/// nothing. Externally tagged on the wire, snake case:
/// `{"record":{…}}`, `{"dependency":{…}}` or `{"service":{…}}`,
/// beside the creator's own `type`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tool {
    /// A tool the client made from a template.
    Record {
        /// The template the tool was made from, by id.
        template: String,
        /// The tool's number among all tools of the client's ever
        /// made from that template: the index its list item's origin
        /// carries. The template and the index name the tool once and
        /// for all.
        index: u64,
        /// The tool's name, as its create gave it, if it gave one.
        /// Absent when it had none.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    /// A transient dependency tool, deployed for an agent's run.
    Dependency {
        /// The agent it was deployed for: see [`Agent`].
        agent: Agent,
        /// The dependency tool template it was deployed from, by id.
        template: String,
    },
    /// A service, made from a dependency tool template under its
    /// scope key.
    Service {
        /// The dependency tool template it was made from, by id.
        template: String,
        /// Whose it is: see [`key::Owner`].
        owner: key::Owner,
    },
}
