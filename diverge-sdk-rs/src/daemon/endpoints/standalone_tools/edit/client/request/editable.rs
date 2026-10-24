//! The tool as a patch sees it.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::daemon::endpoints::agents::create::client::request::{FuseMount, VolumeMount};

/// The members of a tool an edit may change, as one JSON document:
/// what a patch is applied to, and what its result must be again —
/// every member present, a member the patch takes away read as
/// absent or empty, and no member this type does not have. A
/// connected tool's document is its name and nothing else of this
/// caller's: `account` `null` and every mount list empty, which a
/// patch leaves so.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Editable {
    /// The name, unique among the caller's tools; another's is the
    /// edit's `InUse`. `null`, the tool has no name, and is reached
    /// once and for all only.
    #[serde(default)]
    pub name: Option<String>,
    /// The account the tool runs under, by identity: see the create's
    /// [`account`](crate::daemon::endpoints::standalone_tools::create::client::request::Frame::account).
    /// An account the daemon does not have is the edit's `NoAccount`,
    /// one the caller holds no `assign` grant over its `Forbidden`.
    /// `null`, the tool runs under no account, and everything it asks
    /// of the daemon is answered `Forbidden` from then on. Changes
    /// live.
    #[serde(default)]
    pub account: Option<crate::shared::Identity>,
    /// Volumes of the provider the create pinned the tool to, as it
    /// is to mount them: see [`VolumeMount`]. Ordered, and applied in
    /// order; no mount's path, in any list, is a prefix of another's.
    /// A tool pinned to no provider has none, and a patch that gives
    /// it one is the edit's error.
    #[serde(default)]
    pub volume_mounts: Vec<VolumeMount>,
    /// Files of providers' volumes served live across the daemon,
    /// mounted one each over FUSE: see [`FuseMount`].
    #[serde(default)]
    pub fuse_file_mounts: Vec<FuseMount>,
    /// Directories of providers' volumes served live across the
    /// daemon, mounted one each over FUSE: see [`FuseMount`].
    #[serde(default)]
    pub fuse_directory_mounts: Vec<FuseMount>,
    /// Which vault key each secret the template declares is, as the
    /// create's [`secrets`](crate::daemon::create::Inner::secrets)
    /// gave them: every name the template declares and no other, else
    /// the edit's `secrets` error; a key named anew takes the `use`
    /// grant over it, and must be one the vault holds with at least
    /// one value, else the same error. Resolved at the container's
    /// next activation.
    #[serde(default)]
    pub secrets: IndexMap<String, String>,
}
