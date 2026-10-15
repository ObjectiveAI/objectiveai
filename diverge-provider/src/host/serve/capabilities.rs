//! What this provider holds out, read off its configuration.

use diverge_sdk::provider::endpoints::info::server::response::{Capabilities, VolumeCapabilities};

use crate::config::Config;

/// The host's capabilities, each a fact of the host:
///
/// - `images_store`: the `images` section names a client image store,
///   which is the only way the host has one.
/// - `images_registries`: every host of `containers.podman.registries`,
///   the registries the host pulls from with the credentials it holds.
/// - `containers_fuse_mount` and `containers_fuse_serve`: the host
///   mounts every FUSE mount a run names and serves every running
///   container's subtree, through the proxy it places in each.
/// - The three volume modes, each kept, with the most the mode allows:
///   a persistent volume one user at a time — mounted or FUSE served,
///   never twice and never both — an ephemeral or a read-only one by
///   any number of either at once.
pub fn capabilities(config: &Config) -> Capabilities {
    Capabilities {
        images_store: config.images.is_some(),
        images_registries: config
            .containers
            .podman
            .registries
            .iter()
            .map(|registry| registry.host.clone())
            .collect(),
        containers_fuse_mount: true,
        containers_fuse_serve: true,
        volume_persistent: Some(VolumeCapabilities {
            mount: true,
            fuse_serve: true,
            mount_many: false,
            fuse_serve_many: false,
            mount_and_fuse_serve: false,
        }),
        volume_ephemeral: Some(ANY),
        volume_read_only: Some(ANY),
    }
}

/// A mode held by any number of containers and FUSE serves at once.
const ANY: VolumeCapabilities = VolumeCapabilities {
    mount: true,
    fuse_serve: true,
    mount_many: true,
    fuse_serve_many: true,
    mount_and_fuse_serve: true,
};
