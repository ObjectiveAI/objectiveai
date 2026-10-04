//! One tool of the caller's, named either way.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::agents::logs::server::response::Identity;
use super::Itself;

/// One tool of the caller's: by its name, by its template and its
/// index, — a connected tool — by the provider and container id it
/// joined, or — when the caller is a tool — itself. See [`reference`](super) for which names what. Untagged
/// JSON, one object any way; an object with members of more than one
/// variant does not decode.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Tool {
    /// By name: `{"name":…}`.
    Name {
        /// The tool's name, as its create or its connect gave it. A
        /// tool given none is not reached this way.
        name: String,
    },
    /// By template and index: `{"template":…,"index":…}`.
    TemplateIndex {
        /// The template the tool was made from, by id.
        template: String,
        /// The tool's number among all tools of the caller's
        /// ever made from that template, as its list item carries it.
        index: u64,
    },
    /// By what it joined: `{"provider":…,"id":…}`. A connected tool
    /// only, which has no template; the pair names it once and for
    /// all, as the connect named it.
    Connected {
        /// The provider the container runs on, as the connect named
        /// it.
        provider: Identity,
        /// The container's id, as the connect named it.
        id: String,
    },
    /// The caller itself, when the caller is a tool: the string
    /// `"self"`. Names nothing when the caller is a client or an agent.
    Itself(Itself),
}
