//! Asking for a container.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{FuseMount, Image, VolumeMount};

/// Ask a provider to create a container.
///
/// Everything here is what a caller may CHOOSE, and it is the same for
/// every kind of container — the arguments included, which either
/// kind is handed once: what differs between an agent and a tool
/// server is what a caller says into it once it runs, on a channel,
/// not here.
/// What a caller may not choose is not here at all rather than here
/// and ignored — the container's name, its published ports, its
/// entrypoint and its environment are the provider's, because they
/// are how the provider reaches the container and how the container
/// reaches back. There is no environment: the mounts are the caller's
/// only provisioning channel, and a field that is accepted and
/// ignored is a field callers will believe in.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Container {
    /// The image: a name and a digest. See [`Image`].
    pub image: Image,
    /// How much memory the container may have, in BYTES.
    ///
    /// A ceiling, not a hint. A process that exceeds what the
    /// container is allowed is killed by the kernel rather than told —
    /// no failed allocation to catch, no warning first — and the
    /// container will not see this number in its own
    /// `/proc/meminfo`, which reports the host's. An image that sizes
    /// itself off what it thinks it has will size itself wrong.
    ///
    /// Bytes rather than megabytes because a unit that has to be
    /// spelled out in prose is a unit half of everyone gets wrong.
    pub memory: u64,
    /// How much the container may WRITE, in BYTES.
    ///
    /// Its own filesystem only — what it adds to or changes over the
    /// image it came from. The image's layers are read-only and are
    /// not counted, so a container starts at nothing however large the
    /// image is.
    ///
    /// # It does not govern the mounts
    ///
    /// A volume is storage that already existed, with a size of its
    /// own that a [`volume`](crate::endpoints::volumes) stated when it
    /// was made. A number here that silently applied to it would be
    /// this request deciding how much of somebody else's storage a
    /// container may fill.
    ///
    /// Bytes rather than megabytes, for the reason
    /// [`memory`](Self::memory) gives.
    pub disk: u64,
    /// Volumes the provider offers, made visible inside the container.
    ///
    /// Ordered, and a provider applies them in order. See
    /// [`VolumeMount`] for how one is named without a host path. No
    /// mount's path, in any list, is a prefix of another's: mounting
    /// INTO a directory the image owns is the point, and mounts
    /// stacking on each other is not.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub volume_mounts: Vec<VolumeMount>,
    /// Files the caller serves LIVE, mounted one each over FUSE.
    ///
    /// Each names a file by a path, an id of the caller's, and
    /// whether the container may write it — see [`FuseMount`]. The
    /// provider MUST mount every one before the container starts, and
    /// every open and every changed close inside the container is one
    /// ask back to the caller, by that id. The file is overwritten in
    /// place only; a program that replaces its file by rename needs a
    /// directory mount. Its path is no other mount's and lies inside
    /// none, as every mount's.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fuse_file_mounts: Vec<FuseMount>,
    /// Directories the caller serves LIVE, mounted one each over FUSE.
    ///
    /// Each names a directory by a path, an id of the caller's, and
    /// whether the container may change it — see [`FuseMount`]. The
    /// whole tree under the path is the caller's: every listing,
    /// read, write, creation, removal and rename inside the container
    /// is one ask back to the caller, by that id. No other mount may
    /// lie inside it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fuse_directory_mounts: Vec<FuseMount>,
    /// What the image is told once, as the image defines it, for the
    /// container's life.
    ///
    /// A JSON value, because this crate does not know what an image
    /// takes — an agent's model and tools, a tool server's own knobs
    /// — and a wire that typed it would have to be revised for every
    /// image that ever ran. The provider hands it to the container
    /// and does not read it; what the value MAY be is what
    /// [`schema`](crate::shared::containers::schema) answers, for
    /// either kind of container.
    pub arguments: Value,
}
