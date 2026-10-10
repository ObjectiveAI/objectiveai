//! The request a run is: the template and the record's mounts, or a
//! dependency's template and the agent's paths, as the provider
//! protocol states a container.

use diverge_sdk::daemon::endpoints::agents::create::client::request::VolumeMount;
use diverge_sdk::daemon::template::Template;
use diverge_sdk::shared::containers::dependencies;
use diverge_sdk::shared::containers::request;

use super::fuse::Mounts;

/// The container to run: the template's image, limits and arguments;
/// the pinned provider's volumes as the record names them — a mount
/// states which mode it means the volume to have, which the provider
/// holds the volume to, refusing the run for one in another mode;
/// and every FUSE mount under the id the daemon serves it by.
pub fn container<T>(template: &Template<T>, volumes: &[VolumeMount], mounts: &Mounts) -> request::Container {
    request::Container {
        image: request::Image {
            name: template.image.name.clone(),
            digest: template.image.digest.clone(),
        },
        memory: template.memory,
        disk: template.disk,
        volume_mounts: volumes
            .iter()
            .map(|mount| request::VolumeMount {
                volume_name: mount.volume_name.clone(),
                volume_relative_path: mount.volume_relative_path.clone(),
                mode: mount.volume_mode,
                container_path: mount.container_path.clone(),
            })
            .collect(),
        fuse_file_mounts: mounts.files.clone(),
        fuse_directory_mounts: mounts.directories.clone(),
        arguments: template.arguments.clone(),
    }
}

/// The container a dependency is: its template's image, limits and
/// arguments, no volume — a dependency names none — and every mount
/// of the agent's paths under the id the daemon serves it by.
pub fn dependency(template: &dependencies::Template, mounts: &Mounts) -> request::Container {
    request::Container {
        image: request::Image {
            name: template.image.name.clone(),
            digest: template.image.digest.clone(),
        },
        memory: template.memory,
        disk: template.disk,
        volume_mounts: Vec::new(),
        fuse_file_mounts: mounts.files.clone(),
        fuse_directory_mounts: mounts.directories.clone(),
        arguments: template.arguments.clone(),
    }
}
