//! The members an agent's create and a tool's create share.

use serde::{Deserialize, Serialize};

use crate::daemon::daemon_tools::DaemonTools;
use crate::daemon::endpoints::agents::create::client::request::{FuseMount, Provider};
use crate::daemon::reference;

/// Everything a container is made of that its template does not say
/// and its name is not: the template, the provider pin and its
/// volumes, the FUSE mounts, the daemon's own tools, the deployer.
/// Flattened into an [agent's](crate::daemon::endpoints::agents::create)
/// and a [tool's](crate::daemon::endpoints::tools::create) create, so
/// its members are the request's own; the daemon's tools are
/// flattened again inside it, so each tool is a member of the request
/// beside the mounts, as the mounts are.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Inner {
    /// The template the container is made from, by its id — the hash
    /// its family's `templates::create` answered. An id no template
    /// of the caller's has, or one of the other family, is the
    /// create's error. The template's image, limits, resources and
    /// arguments are the container's for its life.
    pub template: String,
    /// The one provider the container runs on, and the volumes of that
    /// provider made visible inside it. See [`Provider`].
    ///
    /// Absent, the container runs on whichever provider the daemon
    /// chooses, and mounts no volume: a volume is a provider's own
    /// and does not carry across, so a container with state on a
    /// provider's disk is a container of that provider. The
    /// container's for its life, and never the template's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<Provider>,
    /// Files of providers' volumes served LIVE across the daemon,
    /// mounted one each over FUSE.
    ///
    /// Each names a file by a provider, a volume of that provider's
    /// and a path in it, and its path in the container — see
    /// [`FuseMount`]; each may name a different provider. Every one
    /// is mounted before the container runs, and every read and every
    /// write of a piece inside the container is one ask, forwarded by
    /// the daemon to the volume's provider and served from the
    /// volume's file in place. The file is
    /// overwritten in place only; a program that replaces its file by
    /// rename needs a directory mount. Its container path is no other
    /// mount's — the template's resource mounts included — and lies
    /// inside none, as every mount's.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fuse_file_mounts: Vec<FuseMount>,
    /// Directories of providers' volumes served LIVE across the
    /// daemon, mounted one each over FUSE.
    ///
    /// Each names a directory by a provider, a volume of that
    /// provider's and a path in it, and its path in the container —
    /// see [`FuseMount`]; each may name a different provider. The
    /// whole tree under the volume path is what the container sees:
    /// every listing, stat, read or write of a piece, truncation,
    /// change of attributes, creation, removal and rename inside the
    /// container is one ask, forwarded by the daemon to the volume's
    /// provider and served from the volume's tree in place. No other
    /// mount may lie inside it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fuse_directory_mounts: Vec<FuseMount>,
    /// The daemon's own tools the container holds, and how far each
    /// reaches: see [`DaemonTools`]. Flattened, so each tool is a
    /// member of the request, every one present and `disabled` when
    /// not held. Which it holds is the container's for its life; how
    /// far each reaches is edited.
    #[serde(flatten)]
    pub daemon_tools: DaemonTools,
    /// The agent of the caller's the daemon hands this container's
    /// declared tool dependencies to — each as the template and the
    /// instructions the container returned at register time — when no
    /// [route](crate::daemon::endpoints::tools::routes) answers them.
    /// The deployer makes the tool, attaches it, and may add a route so
    /// that the next ask at that position is answered without it. By
    /// name, or by template and index: see [`reference::Agent`].
    /// Absent, the daemon deploys nothing itself: a dependency no
    /// route answers is not met, and the container's tools channel is
    /// answered with an error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deployer_agent: Option<reference::Agent>,
}
