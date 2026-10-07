//! The request a run is: the template and the record's mounts as the
//! provider protocol states a container.

use diverge_sdk::daemon::endpoints::agents::create::client::request::VolumeMount;
use diverge_sdk::daemon::template::Template;
use diverge_sdk::shared::containers::request;

use super::fuse::Mounts;

/// The container to run: the template's image, limits and arguments;
/// the pinned provider's volumes as the record names them — a mount
/// states which mode it means the volume to have, and the volume's
/// own mode is the provider's to keep; and every FUSE mount under the
/// id the daemon serves it by.
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
                container_path: mount.container_path.clone(),
            })
            .collect(),
        fuse_file_mounts: mounts.files.clone(),
        fuse_directory_mounts: mounts.directories.clone(),
        arguments: template.arguments.clone(),
    }
}
