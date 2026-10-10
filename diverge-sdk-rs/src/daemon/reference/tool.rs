//! One tool of the caller's, named any of four ways.

use serde::{Deserialize, Serialize};

use super::Agent;
use crate::daemon::endpoints::agents::logs::server::response::Identity;

/// One tool of the caller's: by its name, by its template and its
/// index, by — a connected tool — the provider and container id it
/// joined, or — a dependency tool — the agent it was deployed for and
/// the name its template declared. See [`reference`](super) for which
/// names what. Untagged JSON, one object any way; an object with
/// members of more than one variant does not decode.
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
        /// The tool's number among all tools of the caller's ever made
        /// from that template, as its list item carries it.
        index: u64,
    },
    /// By what it joined: `{"provider":…,"id":…}`. A connected tool
    /// only, which has no template; the pair names it once and for all,
    /// as the connect named it.
    Connected {
        /// The provider the container runs on, as the connect named it.
        provider: Identity,
        /// The container's id, as the connect named it.
        id: String,
    },
    /// By the agent it was deployed for and its declared name:
    /// `{"agent":…,"dependency":…}`. A dependency tool only, which
    /// lives while its agent's container runs: found while it does,
    /// and nothing after.
    Dependency {
        /// The agent, by its name or once and for all: see [`Agent`].
        agent: Agent,
        /// The name the agent's program declared the dependency under.
        dependency: String,
    },
}
