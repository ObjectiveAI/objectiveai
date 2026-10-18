//! Which of the two modes a dependency template declares.

use serde::{Deserialize, Serialize};

use super::{Mount, Scope};

/// A transient tool, or a service: the two things a dependency can be,
/// told apart on the wire by the `mode` member beside the template's
/// other members — `"transient"` or `"service"` — with the members of
/// that mode flat beside it, and no member of the other.
///
/// # A transient tool is the agent's run's
///
/// Deployed when the agent's container starts, from the template as
/// the program declared it, and stopped with the agent's run: it
/// lives exactly as long as one run of one agent, and the next run
/// deploys it again. Its database scope is the agent's — one per
/// parent agent, which it has without saying so — and it alone may
/// mount the agent's own files and directories, since there is one
/// agent whose files they are.
///
/// # A service is the caller's
///
/// Made once, when the first agent that declares it registers, under
/// the scope key its [`Scope`] names — the agent, the agent's
/// template, or the template alone — and kept by the caller from then
/// on: started again whenever its container ends, and at every start
/// of the caller's, for as long as any agent is attached to it. An
/// agent is attached by a registration that declares the template and
/// detached by one that no longer does, and a service with no agent
/// attached goes down for good after the caller's idle time, an
/// attachment within that time keeping it. Nothing else makes a
/// service inactive: it serves no agent's run in particular, so no
/// run's end is its end. It mounts nothing of any agent's, since it
/// is many agents'. Its database scope is shared as far as it is, and
/// lives as long as its scope's owner: the agent, the agent template,
/// or — for a global service — the last agent template whose agents
/// ever attached.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum Mode {
    /// A tool for the agent's run: `"mode":"transient"`.
    Transient {
        /// Files of the agent container served live into the tool
        /// container, one each over FUSE: see [`Mount`]. The agent's
        /// path names a file; the tool's path is where it appears.
        /// Absent when empty.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        fuse_file_mounts: Vec<Mount>,
        /// Directories of the agent container served live into the
        /// tool container, one each over FUSE: see [`Mount`]. The
        /// agent's path names a directory, whose whole tree the tool
        /// sees; the tool's path is where it appears. Absent when
        /// empty.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        fuse_directory_mounts: Vec<Mount>,
    },
    /// A service the caller keeps: `"mode":"service"`.
    Service {
        /// How far the service is shared, and its database with it:
        /// see [`Scope`].
        scope: Scope,
    },
}
