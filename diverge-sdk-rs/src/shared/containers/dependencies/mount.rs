//! One file, or one directory, of the agent container served live
//! into the tool container.

use serde::{Deserialize, Serialize};

/// A path of the agent container, mounted at a path of the tool
/// container over FUSE: source before destination, the order a mount
/// reads in everywhere else. Which of the two it is — a file or a
/// directory — is which list of the [`Template`](super::Template) it
/// is on.
///
/// The caller serves the agent container's path live — a
/// `containers::serve` of it, or of the directory holding a file —
/// and makes the mount in the tool container as a FUSE mount answered
/// from that serve, so every read, write, listing, removal, rename and
/// new directory the tool makes is one ask answered from the agent's
/// own filesystem, where the agent sees it at once. Nothing is copied:
/// the two containers share the path for as long as both run.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Mount {
    /// Where it is in the agent container, as path components from the
    /// agent container's root — the shape every path in this crate
    /// takes. No component is empty, `.` or `..`, and the path is not
    /// the root: a file on the file list, a directory on the directory
    /// list.
    pub agent_path: Vec<String>,
    /// Where it appears in the tool container, as path components from
    /// the tool container's root, under the rules a
    /// [`Container`](crate::shared::containers::request::Container)
    /// states for its mounts: not the root, no other mount's, inside
    /// no other mount's, and no other mount's inside it.
    pub tool_path: Vec<String>,
}
