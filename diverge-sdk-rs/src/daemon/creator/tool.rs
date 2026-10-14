//! A tool as a creator.

use serde::{Deserialize, Serialize};

use crate::daemon::key;

/// A tool of the client's that made something: one the client made
/// from a template, named once and for all by its template and its
/// index, and by its name as it was called; or a dependency tool,
/// named once and for all by the template it runs from and whose it
/// is. A connected tool makes nothing. Externally tagged on the wire,
/// snake case: `{"record":{…}}` or `{"dependency":{…}}`, beside the
/// creator's own `type`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
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
    /// A dependency tool, running from a template an agent's program
    /// declared.
    Dependency {
        /// The dependency template it runs from, by id.
        template: String,
        /// Whose it is: see [`key::Owner`].
        owner: key::Owner,
    },
}
