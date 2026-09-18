//! The podman the provider runs containers with.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::Registry;

/// Everything the provider hands podman: where it pulls from, where
/// it keeps its data, and how much of the host it may take.
///
/// Every field is required when the section is present. Absent, the
/// section is its [`Default`]: what a provider runs with before it
/// has written a line of configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Podman {
    /// The registries the provider looks in, all at once and beside
    /// the caller, for an image it does not hold itself, each with the
    /// credential the provider presents to it, if any; the first to
    /// have the image is pulled from. The provider's own registry,
    /// which serves what a caller holds, is not listed here and needs
    /// no entry.
    pub registries: Vec<Registry>,
    /// The directory podman's data is kept under: the image cache,
    /// what a container writes over its image, and what holds a
    /// volume's changes apart while `persist` is `false`. On Linux it
    /// is podman's storage root, the `graphroot`: every podman
    /// invocation the provider makes is given it as `--root`, so it
    /// is the whole store the provider sees, and an image the provider
    /// holds itself is loaded into it, not into podman's default
    /// store; a changed
    /// path is a fresh, empty store, and the old one is left as it
    /// was. On macOS and Windows it is where the podman machine
    /// lives, description and disk: the provider makes the machine
    /// under this path, a changed path is a fresh machine under the
    /// new one, and the machine under the old path is left as it
    /// was, for the operator to stop or remove when they choose; the
    /// data lies inside the machine's disk. A path that is not
    /// absolute is resolved relative to the directory that contains
    /// `config.yaml` itself, never to the working directory; an
    /// absolute path stands as written.
    ///
    /// A container's `disk` is enforced as podman's storage size
    /// option on the container, which the overlay driver keeps with
    /// the kernel's project quotas. So the filesystem under this path
    /// has them: XFS mounted with `pquota`, or ext4 with project
    /// quota enabled. On any other filesystem podman refuses the
    /// option, and every run fails with its refusal; the provider
    /// does not probe for it.
    pub storage_path: PathBuf,
    /// The most the image cache may hold, in BYTES: the layers of
    /// every image pulled, kept for the next run of it. The provider
    /// removes images no running container uses to stay under it,
    /// and an image larger than it alone cannot be pulled.
    pub image_cache_disk: u64,
    /// The most the running containers may write between them, in
    /// BYTES: the sum of every running container's requested `disk`
    /// never exceeds it, and a run that would take it over is
    /// refused. What a container writes is its own layer over the
    /// image; the image is not counted here.
    pub container_overlay_disk: u64,
    /// The most memory the running containers may hold between them,
    /// in BYTES: the sum of every running container's `memory` never
    /// exceeds it, and a run that would take it over is refused. On
    /// macOS the podman machine is given exactly this much when the
    /// provider starts, since a machine there holds its memory from
    /// the host; on Windows the machine's memory is WSL's to give and
    /// is not set.
    pub memory: u64,
}

/// What a provider runs with before it has written a line of
/// configuration: the three registries a caller may name, `docker.io`
/// first, each pulled from anonymously; podman's data under
/// `podman_data` beside `config.yaml`; a 32 GiB image cache; 32 GiB
/// of overlay disk; and 8 GiB of memory.
impl Default for Podman {
    fn default() -> Self {
        Podman {
            registries: ["docker.io", "ghcr.io", "quay.io"]
                .iter()
                .map(|host| Registry {
                    host: host.to_string(),
                    credential: None,
                })
                .collect(),
            storage_path: PathBuf::from("podman_data"),
            // 32 GiB.
            image_cache_disk: 32 * 1024 * 1024 * 1024,
            // 32 GiB.
            container_overlay_disk: 32 * 1024 * 1024 * 1024,
            // 8 GiB.
            memory: 8 * 1024 * 1024 * 1024,
        }
    }
}
