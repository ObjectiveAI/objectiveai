//! The store of content mounted by identity.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// What the provider holds for identity mounts: every file and
/// directory a caller had it fetch, kept under the identity that
/// names it, verified, and mounted into the next run that names it
/// again.
///
/// Every field is required when the section is present. Absent, the
/// section is its [`Default`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    /// The directory the content is kept under. A path that is not
    /// absolute is resolved relative to the directory that contains
    /// `config.yaml` itself, never to the working directory; an
    /// absolute path stands as written.
    pub storage_path: PathBuf,
    /// The most the store may hold, in BYTES. The provider removes
    /// content no running container mounts to stay under it, and
    /// content larger than it alone cannot be held.
    pub disk: u64,
}

/// What a provider runs with before it has written a line of
/// configuration: the content under `identity` beside `config.yaml`,
/// and 8 GiB of it.
impl Default for Identity {
    fn default() -> Self {
        Identity {
            storage_path: PathBuf::from("identity"),
            // 8 GiB.
            disk: 8 * 1024 * 1024 * 1024,
        }
    }
}
