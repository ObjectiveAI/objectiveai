//! The members an agent's edit and a tool's edit share.

use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::agents::create::client::request::{FuseMount, VolumeMount};
use crate::daemon::reference;
use super::Change;

/// Everything about a container that changes after its create, every
/// member an optional [`Change`]. A member absent leaves that of the
/// container as it is; `delete` takes it away; `set` replaces it whole
/// — a list of mounts is the new list entire, the name the new name —
/// and nothing is merged. A request with every member absent changes
/// nothing and is not a failure. What is not here does not change: the
/// template, the provider pin, the index, who made it.
///
/// # What waits for the container to be inactive
///
/// The mounts: a request that names any of the three mount lists is
/// refused while the container is active, and left as it is. The name,
/// the account and the deployer change live.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Edit {
    /// The name, unique among the caller's agents or tools as the
    /// container is one or the other; a name another holds is the
    /// edit's `InUse`, and nothing changes. Absent, as it is; `delete`,
    /// the container has no name, and is reached once and for all only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<Change<String>>,
    /// The account the container runs under: see the create's
    /// [`account`](crate::daemon::create::Inner::account). An account
    /// the daemon does not have is the edit's `NoAccount`, and nothing
    /// changes. Absent, as it is; `delete`, the container runs under no
    /// account, and runs no command of the daemon's from then on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account: Option<Change<String>>,
    /// Volumes of the provider the create pinned the container to, as
    /// it is to mount them: see [`VolumeMount`]. The new list whole; a
    /// container pinned to no provider mounts no volume, and a request
    /// naming one for it is the edit's error. Ordered, and applied in
    /// order; no mount's path, in any list, is a prefix of another's.
    /// Absent, as it is; `delete`, no volume mounted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume_mounts: Option<Change<Vec<VolumeMount>>>,
    /// Files of providers' volumes served live across the daemon,
    /// mounted one each over FUSE, as the container is to mount them:
    /// see [`FuseMount`], and the create's
    /// [`fuse_file_mounts`](crate::daemon::create::Inner::fuse_file_mounts).
    /// The new list whole. Absent, as it is; `delete`, none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fuse_file_mounts: Option<Change<Vec<FuseMount>>>,
    /// Directories of providers' volumes served live across the daemon,
    /// mounted one each over FUSE, as the container is to mount them:
    /// see [`FuseMount`], and the create's
    /// [`fuse_directory_mounts`](crate::daemon::create::Inner::fuse_directory_mounts).
    /// The new list whole. Absent, as it is; `delete`, none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fuse_directory_mounts: Option<Change<Vec<FuseMount>>>,
    /// The deployer: see the create's
    /// [`deployer_agent`](crate::daemon::create::Inner::deployer_agent).
    /// Absent, as it is; `delete`, none, and the container's
    /// dependencies are routed or unmet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deployer_agent: Option<Change<reference::Agent>>,
}
