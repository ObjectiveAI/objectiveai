//! The store of images clients build.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Where the images clients build on the provider are kept, and how
/// many bytes they may take between them.
///
/// A directory and a number, as a volume store is. Under the
/// directory the provider keeps one pool per identity, named by a
/// hash of the identity, as a registry keeps images: every blob of
/// every image the identity built, content-addressed, a blob two of
/// its images share stored once, and a record per image with its
/// Containerfile and the manifest digest it is known by. The
/// provider's own registry serves each pool, and a run of a client
/// image pulls it from there into the provider's cache of pulled
/// images, where the copy is evicted as any cached image is; the
/// pool is not. Nothing in the store is evicted: an image stays until
/// the identity that built it deletes it, and a build the store has
/// no room for is refused with the answer, not built.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Images {
    /// The directory the pools live under, made by the provider at
    /// its start if it is not there; a relative path is resolved
    /// against the root, the directory that holds `provider.yaml`. On
    /// macOS the podman machine is made seeing it, since a build's
    /// export is written through podman.
    pub path: PathBuf,
    /// The most every pool may hold together, in BYTES: every blob
    /// of every identity's images, each counted once. A build that
    /// leaves the pools over it is undone and answered insufficient
    /// capacity. `0` is refused at the provider's start.
    pub capacity: u64,
}
