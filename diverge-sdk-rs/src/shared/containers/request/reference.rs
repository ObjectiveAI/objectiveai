//! Where an image's bytes may be fetched: a registry and a path there.

use serde::{Deserialize, Serialize};

/// One place the bytes an image's digest names may be fetched from: a
/// registry, and the repository path there. A reference locates and
/// does not identify — the digest does — so an image may carry none,
/// one or several, and two images with one digest are one image
/// whatever each lists. A provider asks a referenced registry only
/// where its own policy lists that registry, with the credential it
/// holds for it, and ignores a reference naming one it does not.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Reference {
    /// The registry's host, as a reference names it before its first
    /// `/`: `docker.io`, `ghcr.io`, `registry.example.com:5000`. No
    /// scheme and no path.
    pub registry: String,
    /// The repository path at that registry — `library/nginx`,
    /// `myorg/myimage`. No host and no tag: the image is
    /// `<registry>/<name>@<digest>` there.
    ///
    /// It lands in a reference by concatenation wherever a provider
    /// pulls from, so a provider refuses one that is not a repository
    /// path and normalizes nothing.
    pub name: String,
}
