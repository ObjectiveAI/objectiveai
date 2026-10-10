//! One tool of the caller's, named any of four ways.

use serde::{Deserialize, Serialize};

use super::Agent;

/// One tool of the caller's: by its name, by its template and its
/// index, by — a connected tool — the daemon and the tool it joined,
/// or — a dependency tool — the agent it was deployed for and the
/// template it was deployed from. See [`reference`](super) for which
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
    /// By what it joined: `{"daemon":…,"tool":…}`. A connected tool
    /// only, which has no template; the pair names it once and for
    /// all, as the connect named it.
    Connected {
        /// The daemon the tool is on, by the name of its record.
        daemon: String,
        /// The tool, as that daemon names it: any of these, read by
        /// that daemon and compared by this one.
        tool: Box<Tool>,
    },
    /// By the agent it was deployed for and its template:
    /// `{"agent":…,"template":…}`. A dependency tool only, which
    /// lives while its agent's container runs: found while it does,
    /// and nothing after.
    Dependency {
        /// The agent, by its name or once and for all: see [`Agent`].
        agent: Agent,
        /// The dependency tool template it was deployed from, by id:
        /// the hash of the template's canonical bytes.
        template: String,
    },
}
