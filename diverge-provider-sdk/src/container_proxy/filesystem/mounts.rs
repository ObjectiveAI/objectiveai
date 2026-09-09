//! FUSE mounts: one file in the container that can be read and —
//! unless read-only — overwritten, but never moved or deleted, and
//! how the server says so.

use std::fmt;

use serde::{Deserialize, Serialize};

/// The environment variable the server sets on the proxy: the FUSE
/// mounts, as a [`Mounts`] in JSON. Unset is no mounts.
pub const MOUNTS_ENV: &str = "DIVERGE_CONTAINER_PROXY_FILESYSTEM_MOUNTS";

/// One file, mounted at one path, its bytes the caller's under one id.
///
/// The proxy's side of a
/// [`FuseMount`](crate::shared::containers::request::FuseMount) on the
/// container request: the server hands each one down as this, and
/// the proxy mounts a FUSE filesystem of exactly one regular file at
/// the path — the mount point is the file itself, and the directory
/// around it stays the image's own. The file can be read and, unless
/// read-only, overwritten; it cannot be moved or deleted. Its bytes
/// are the caller's: a read is a [`fuse::read`](super::super::fuse::read)
/// ask by the id, a write a [`fuse::write`](super::super::fuse::write),
/// and the file's size is the answer's length. The [module](super)
/// states the semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mount {
    /// The file's path, as components from the container's root — the
    /// shape every path in this crate takes.
    pub path: Vec<String>,
    /// The caller's id for the file, echoed on every ask.
    pub id: String,
    /// Whether every write is refused.
    #[serde(default)]
    pub readonly: bool,
}

/// Every FUSE mount the proxy makes at its start.
///
/// The variable's value is a JSON array of [`Mount`]s, which is what
/// serde makes of this newtype. A value that does not parse is an
/// ERROR the proxy refuses to start over — unlike the filetree's
/// ignore list, whose absence only makes the tree larger than meant,
/// a credential file that is missing makes the agent fail somewhere
/// far from the cause.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Mounts(pub Vec<Mount>);

impl Mounts {
    /// Read the variable's value: the JSON above, or the error.
    pub fn parse(value: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(value)
    }
}

/// The variable's value: the JSON the proxy parses.
impl fmt::Display for Mounts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let json = serde_json::to_string(&self.0).map_err(|_| fmt::Error)?;
        f.write_str(&json)
    }
}
