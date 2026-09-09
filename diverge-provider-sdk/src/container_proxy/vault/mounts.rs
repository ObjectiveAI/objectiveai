//! Vault mounts: a key as one file in the container, and how the
//! server says so.

use std::fmt;

use serde::{Deserialize, Serialize};

/// The environment variable the server sets on the proxy: the vault
/// mounts, as a [`Mounts`] in JSON. Unset is no mounts.
pub const MOUNTS_ENV: &str = "DIVERGE_CONTAINER_PROXY_VAULT_MOUNTS";

/// One vault key, served as one regular file at one path.
///
/// The proxy mounts a FUSE filesystem of exactly one file at the
/// path — the mount point is the file itself, and the directory
/// around it stays the image's own — whose bytes are the key's value:
/// a read is a `get`, a write is a `set`, and the file's size is the
/// value's length. The [module](super) states the semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mount {
    /// The file's path, as components from the container's root — the
    /// shape every path in this crate takes.
    pub path: Vec<String>,
    /// The vault key whose value the file is.
    pub key: String,
}

/// Every vault mount the proxy makes at its start.
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
