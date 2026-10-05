//! One agent of the caller's, named either way.

use serde::{Deserialize, Serialize};

/// One agent of the caller's: by its name, by its template and its
/// index. See [`reference`](super) for which names what. Untagged JSON,
/// one object either way; an object with members of both variants does
/// not decode.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Agent {
    /// By name: `{"name":…}`.
    Name {
        /// The agent's name, as its create gave it. An agent given none
        /// is not reached this way.
        name: String,
    },
    /// By template and index: `{"template":…,"index":…}`.
    TemplateIndex {
        /// The template the agent was made from, by id.
        template: String,
        /// The agent's number among all agents of the caller's ever
        /// made from that template, as its list item carries it.
        index: u64,
    },
}
