//! The store of images clients build.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Where the images clients build on the provider are kept, and how
/// many bytes they may take between them.
///
/// A directory and a number, as a volume store is. Under the
/// directory the provider keeps a podman storage root of its own,
/// `root/`, which holds every client image and which the provider's
/// cache of pulled images never touches, and `meta/`, where each
/// image's Containerfile and record are kept by identity and name.
/// Nothing in the store is evicted: an image stays until the identity
/// that built it deletes it, and a build the store has no room for is
/// refused with the answer, not built.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Images {
    /// The directory the store lives under, made by the provider at
    /// its start if it is not there; a relative path is resolved
    /// against the root, the directory that holds `config.yaml`. On
    /// macOS the podman machine is made seeing it, since the builds
    /// run inside the machine.
    pub path: PathBuf,
    /// The most the store may hold, in BYTES: every client image's
    /// size, every layer counted, shared or not, as the cache is
    /// counted. A build that leaves the store over it is undone and
    /// answered insufficient capacity. `0` is refused at the
    /// provider's start.
    pub capacity: u64,
}
