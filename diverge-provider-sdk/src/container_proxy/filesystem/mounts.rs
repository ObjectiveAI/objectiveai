//! FUSE mounts: the files and directories the caller serves live, and
//! how the server says so.

use std::fmt;

use serde::{Deserialize, Serialize};

/// The environment variable the server sets on the proxy: the FUSE
/// mounts, as a [`Mounts`] in JSON. Unset is no mounts.
pub const MOUNTS_ENV: &str = "DIVERGE_CONTAINER_PROXY_FILESYSTEM_MOUNTS";

/// One mount, at one path, its contents the caller's under one id —
/// a file or a directory, by which list of [`Mounts`] it is on.
///
/// The proxy's side of a
/// [`FuseMount`](crate::shared::containers::request::FuseMount) on the
/// container request: the server hands each one down as this, and
/// the proxy mounts a FUSE filesystem at the path — of exactly one
/// regular file, or of a directory tree — whose contents are the
/// caller's: every read is a [`fuse::read`](super::super::fuse::read)
/// ask by the id, every write a [`fuse::write`](super::super::fuse::write),
/// and a directory's listings, removals, renames and new directories
/// their own asks. The [module](super) states the semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mount {
    /// The mount's path, as components from the container's root —
    /// the shape every path in this crate takes.
    pub path: Vec<String>,
    /// The caller's id for the mount, echoed on every ask.
    pub id: String,
    /// Whether every mutation is refused.
    #[serde(default)]
    pub readonly: bool,
}

/// Every FUSE mount the proxy makes at its start, files and
/// directories.
///
/// The variable's value is this as a JSON object, `{"files": [...],
/// "directories": [...]}`, each a list of [`Mount`]s; either may be
/// absent. A value that does not parse is an ERROR the proxy refuses
/// to start over — unlike the filetree's ignore list, whose absence
/// only makes the tree larger than meant, a credential file that is
/// missing makes the agent fail somewhere far from the cause.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mounts {
    /// The file mounts: one regular file each.
    #[serde(default)]
    pub files: Vec<Mount>,
    /// The directory mounts: a tree each.
    #[serde(default)]
    pub directories: Vec<Mount>,
}

impl Mounts {
    /// Read the variable's value: the JSON above, or the error.
    pub fn parse(value: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(value)
    }
}

/// The variable's value: the JSON the proxy parses.
impl fmt::Display for Mounts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let json = serde_json::to_string(self).map_err(|_| fmt::Error)?;
        f.write_str(&json)
    }
}
