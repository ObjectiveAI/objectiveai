//! What a run request is refused for before anything is held.

use super::render;
use crate::shared::containers::request::Container;
use crate::shared::error::Error;

/// One mount of the request, whichever list it is on.
struct Named<'a> {
    /// Where it appears inside the container.
    path: &'a [String],
    /// The caller's id, for a FUSE mount; `None` for a volume.
    id: Option<&'a str>,
}

/// The refusals the specification puts before the volumes are held:
/// a mount at the container's root; a component of any mount's path
/// that is not a name — empty, `.` or `..` — in a container path or
/// a volume's relative path; two mounts with one path; a mount
/// inside another mount, a volume mount inside a FUSE mount no more
/// than a FUSE mount inside a volume mount; two FUSE mounts with one
/// id. The first found is the run's error, and nothing is held or
/// deployed for it.
pub(crate) fn check(request: &Container) -> Result<(), Error> {
    let mounts: Vec<Named<'_>> = request
        .volume_mounts
        .iter()
        .map(|mount| Named {
            path: &mount.container_path,
            id: None,
        })
        .chain(
            request
                .fuse_file_mounts
                .iter()
                .chain(&request.fuse_directory_mounts)
                .map(|mount| Named {
                    path: &mount.container_path,
                    id: Some(&mount.id),
                }),
        )
        .collect();
    for mount in &mounts {
        if mount.path.is_empty() {
            return Err(render::path_refused("a mount at the container's root"));
        }
    }
    for component in mounts
        .iter()
        .flat_map(|mount| mount.path.iter())
        .chain(request.volume_mounts.iter().flat_map(|mount| mount.host_relative_path.iter()))
    {
        if !name(component) {
            return Err(render::path_refused(&format!("the component `{component}` is not a name")));
        }
    }
    for (index, mount) in mounts.iter().enumerate() {
        for other in &mounts[index + 1..] {
            if mount.path == other.path {
                return Err(render::path_refused(&format!("two mounts at `/{}`", mount.path.join("/"))));
            }
            if mount.path.starts_with(other.path) || other.path.starts_with(mount.path) {
                return Err(render::path_refused(&format!(
                    "the mount at `/{}` lies inside the mount at `/{}`",
                    longer(mount.path, other.path).join("/"),
                    shorter(mount.path, other.path).join("/")
                )));
            }
            if let (Some(id), Some(other_id)) = (mount.id, other.id)
                && id == other_id
            {
                return Err(render::id_refused(id));
            }
        }
    }
    Ok(())
}

/// Whether a component is a name: not empty, not `.`, not `..`.
fn name(component: &str) -> bool {
    !component.is_empty() && component != "." && component != ".."
}

/// The longer of two paths, one a prefix of the other.
fn longer<'a>(a: &'a [String], b: &'a [String]) -> &'a [String] {
    if a.len() >= b.len() { a } else { b }
}

/// The shorter of two paths, one a prefix of the other.
fn shorter<'a>(a: &'a [String], b: &'a [String]) -> &'a [String] {
    if a.len() >= b.len() { b } else { a }
}
