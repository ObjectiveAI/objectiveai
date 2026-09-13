//! The section itself, and its defaults.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// The `containers` section: what the running containers may reach
/// between them, where their storage lives, and which registries a
/// caller may name.
///
/// Every field is required when the section is present. The section
/// as a whole may be absent, and then it is [`Default`]: the values
/// below, which are what a provider runs with before it has written
/// a line of configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Containers {
    /// The most memory the running containers may hold between them,
    /// in BYTES: the sum of every running container's `memory` never
    /// exceeds it, and a run that would take it over is refused.
    pub memory: u64,
    /// The most the running containers may write between them, in
    /// BYTES: the sum of every running container's requested `disk`
    /// never exceeds it, and a run that would take it over is
    /// refused. What a container writes is its own layer over the
    /// image; the image is not counted here.
    pub container_overlay_disk: u64,
    /// The most the image cache may hold, in BYTES: the layers of
    /// every image pulled, kept for the next run of it. The provider
    /// removes images no running container uses to stay under it,
    /// and an image larger than it alone cannot be pulled.
    pub image_cache_disk: u64,
    /// The directory container storage is kept under: what a
    /// container writes over its image, and what holds a volume's
    /// changes apart while `persist` is `false`. Resolved relative to
    /// the provider's directory, as every path in the file is; an
    /// absolute path stands as written.
    pub path: PathBuf,
    /// The registries a caller may pull from by naming them in an
    /// image reference: host names, `docker.io`, `ghcr.io`, with no
    /// scheme and no path. A reference whose host is not listed is
    /// refused, and the run with it; a reference naming no host is
    /// read as the first entry's. The provider's own registry, which
    /// serves the images a caller holds, is not listed here and needs
    /// no entry.
    pub registries: Vec<String>,
}

/// What a provider runs with before it has written a line of
/// configuration: 8 GiB of memory, 32 GiB of overlay disk, a 32 GiB
/// image cache, `data/containers` under the provider's directory, and
/// the three registries a caller may name, `docker.io` first.
impl Default for Containers {
    fn default() -> Self {
        Containers {
            memory: 8 * 1024 * 1024 * 1024,
            container_overlay_disk: 32 * 1024 * 1024 * 1024,
            image_cache_disk: 32 * 1024 * 1024 * 1024,
            path: PathBuf::from("data/containers"),
            registries: ["docker.io", "ghcr.io", "quay.io"]
                .iter()
                .map(|host| host.to_string())
                .collect(),
        }
    }
}
