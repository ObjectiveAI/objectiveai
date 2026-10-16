//! The request a run is: the template and the record's mounts, a
//! transient dependency's template and the agent's paths, or a
//! service's template alone, as the provider protocol states a
//! container.

use diverge_sdk::daemon::endpoints::agents::create::client::request::VolumeMount;
use diverge_sdk::daemon::endpoints::agents::create::client::request::Image as TemplateImage;
use diverge_sdk::daemon::template::Template;
use diverge_sdk::shared::containers::dependencies;
use diverge_sdk::shared::containers::request;
use indexmap::IndexMap;
use serde_json::Value;

use super::fuse::Mounts;

/// The container to run: the template's image, limits and arguments,
/// with the template's id, which the container is told at its
/// registration; the pinned provider's volumes as the record names
/// them — a mount states which mode it means the volume to have,
/// which the provider holds the volume to, refusing the run for one
/// in another mode; every FUSE mount under the id the daemon serves
/// it by; and the secrets, each name the template declares to the
/// value its vault key holds now.
pub async fn container<T>(template_id: &str, template: &Template<T>, volumes: &[VolumeMount], mounts: &Mounts, secrets: IndexMap<String, Value>) -> request::Container {
    let (fuse_file_mounts, fuse_directory_mounts) = mounts.lists().await;
    request::Container {
        image: image(&template.image),
        template: template_id.to_string(),
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
        fuse_file_mounts,
        fuse_directory_mounts,
        arguments: template.arguments.clone(),
        secrets,
    }
}

/// The container a transient dependency is: its template's image,
/// limits and arguments, with the template's id, no volume — a
/// dependency names none — every mount of the agent's paths under
/// the id the daemon serves it by, and the secrets, each name the
/// template declares to the value the agent it is deployed for binds
/// it to.
pub async fn dependency(template_id: &str, template: &dependencies::Template, mounts: &Mounts, secrets: IndexMap<String, Value>) -> request::Container {
    let (fuse_file_mounts, fuse_directory_mounts) = mounts.lists().await;
    request::Container {
        image: template.image.clone(),
        template: template_id.to_string(),
        memory: template.memory,
        disk: template.disk,
        volume_mounts: Vec::new(),
        fuse_file_mounts,
        fuse_directory_mounts,
        arguments: template.arguments.clone(),
        secrets,
    }
}

/// The container a service is: its template's image, limits and
/// arguments, with the template's id, no mount of any kind — a
/// service is many agents' and names no volume — and the secrets,
/// each name the template declares to the value the agent that
/// spawns this run binds it to.
pub fn service(template_id: &str, template: &dependencies::Template, secrets: IndexMap<String, Value>) -> request::Container {
    request::Container {
        image: template.image.clone(),
        template: template_id.to_string(),
        memory: template.memory,
        disk: template.disk,
        volume_mounts: Vec::new(),
        fuse_file_mounts: Vec::new(),
        fuse_directory_mounts: Vec::new(),
        arguments: template.arguments.clone(),
        secrets,
    }
}

/// A template's image as a run names it: the digest and every
/// reference, which the chooser judges a provider by before the
/// container is built — see
/// [`capable::registry`](super::capable::registry).
pub fn image(image: &TemplateImage) -> request::Image {
    request::Image {
        digest: image.digest.clone(),
        references: image
            .references
            .iter()
            .map(|reference| request::Reference {
                registry: reference.registry.clone(),
                name: reference.name.clone(),
            })
            .collect(),
    }
}
