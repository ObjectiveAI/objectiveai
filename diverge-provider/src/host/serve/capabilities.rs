//! What this provider holds out, read off its configuration.

use diverge_sdk::provider::endpoints::info::server::response::{Capabilities, MessagesCapabilities, VolumeCapabilities};

use crate::config::Config;

/// The host's capabilities, each a fact of the host:
///
/// - `containers`: `containers.enabled`, the configuration's word on
///   whether the host runs containers at all; `false` is every agents
///   run and tools run answered `Unsupported`.
/// - `images_store`: the `images` section names a Store,
///   which is the only way the host has one.
/// - `images_registries`: every host of `containers.podman.registries`,
///   the registries the host pulls from with the credentials it holds.
/// - The three volume modes, each kept, with the most the mode allows:
///   a persistent volume mounted by any number of containers at once
///   — its image is loop-mounted once and every container binds that
///   one directory — but FUSE served once and never beside a
///   container, since a serve opens the image in userspace and the
///   image bears one writer; an ephemeral or a read-only one by any
///   number of either at once.
/// - `messages`: the `messages` section's rate, as it is written; the
///   section absent, no object, and every listen and send answered
///   `Unsupported`.
pub fn capabilities(config: &Config) -> Capabilities {
    Capabilities {
        containers: config.containers.enabled,
        images_store: config.images.is_some(),
        images_registries: config
            .containers
            .podman
            .registries
            .iter()
            .map(|registry| registry.host.clone())
            .collect(),
        volumes_persistent: VolumeCapabilities {
            mount: true,
            fuse_serve: true,
            mount_many: true,
            fuse_serve_many: false,
            mount_and_fuse_serve: false,
        },
        volumes_ephemeral: ANY,
        volumes_read_only: ANY,
        messages: config.messages.map(|messages| MessagesCapabilities { messages: messages.messages, seconds: messages.seconds }),
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
