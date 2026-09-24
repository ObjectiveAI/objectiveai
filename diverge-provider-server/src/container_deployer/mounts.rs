//! The volume mounts of a deploy: checked, resolved, and put where
//! podman can bind them.

use std::path::Path;

use diverge_provider_sdk::server::mount::Mount;
use diverge_provider_sdk::server::volume_manager::VolumeManager as _;
use futures_util::future;

use super::{ContainerDeployer, Error};
use crate::volume_manager::Volume;

/// One mount made ready: the `--volume` argument podman is handed,
/// and the volume it binds, attached, to be detached when the
/// container is gone.
#[derive(Debug, Clone)]
pub struct Bound {
    /// `<host>:<container>` with `:O` after it for a volume whose
    /// mode is not persist: podman's overlay, the container's writes
    /// discarded with it.
    pub argument: String,
    /// The volume, attached once for this mount.
    pub volume: Volume,
}

/// Every mount checked and made ready, each resolved beside every
/// other. A mount that could not be made undoes the ones that were,
/// and the first failure is the error. Nothing is asked of podman.
pub async fn bind(deployer: &ContainerDeployer, mounts: &[Mount]) -> Result<Vec<Bound>, Error> {
    check(mounts)?;
    let outcomes = future::join_all(mounts.iter().map(|mount| one(deployer, mount))).await;
    let mut bound = Vec::with_capacity(outcomes.len());
    let mut failed = None;
    for outcome in outcomes {
        match outcome {
            Ok(one) => bound.push(one),
            Err(error) => failed = failed.or(Some(error)),
        }
    }
    match failed {
        None => Ok(bound),
        Some(error) => {
            release(&bound).await;
            Err(error)
        }
    }
}

/// Every volume of `bound` detached, each beside every other: the
/// last container to have a stored volume unmounts its image.
pub async fn release(bound: &[Bound]) {
    future::join_all(bound.iter().map(|one| one.volume.detach())).await;
}

/// The refusals the contract puts on the provider, for what a
/// deployment carries: the root as a container path; a component that
/// is not a name, in either path; two mounts at one path; a path that
/// is a prefix of another's.
fn check(mounts: &[Mount]) -> Result<(), Error> {
    for mount in mounts {
        if mount.container_path.is_empty() {
            return Err(Error::Path("the container's root".to_string()));
        }
        for component in mount.container_path.iter().chain(&mount.host_relative_path) {
            if !component_ok(component) {
                return Err(Error::Path(format!("the component `{component}` is not a name")));
            }
        }
    }
    for (index, mount) in mounts.iter().enumerate() {
        for other in &mounts[index + 1..] {
            if mount.container_path == other.container_path {
                return Err(Error::Path(format!("two mounts at `/{}`", mount.container_path.join("/"))));
            }
            if mount.container_path.starts_with(&other.container_path)
                || other.container_path.starts_with(&mount.container_path)
            {
                return Err(Error::Path(format!(
                    "`/{}` lies inside `/{}`",
                    mount.container_path.join("/"),
                    other.container_path.join("/")
                )));
            }
        }
    }
    Ok(())
}

/// Whether a path component is a name: not empty, not `.` or `..`,
/// and without a `/` or a NUL in it.
fn component_ok(component: &str) -> bool {
    !component.is_empty() && component != "." && component != ".." && !component.contains(['/', '\0'])
}

/// One mount resolved: the volume looked up for its identity and
/// attached — its image loop-mounted if this is the first container
/// to have it — the relative path descended on the directory it
/// answers, and the argument built, under podman's overlay when the
/// volume's mode is not persist.
async fn one(deployer: &ContainerDeployer, mount: &Mount) -> Result<Bound, Error> {
    let volume = deployer
        .volumes
        .get(&mount.client_identity, &mount.host_name)
        .await?
        .ok_or_else(|| Error::Volume(mount.host_name.clone()))?;
    let dir = volume.attach(&deployer.mounts_dir).await.map_err(Error::Mount)?;
    let host = descend(&dir, &mount.host_relative_path);
    let mut argument = format!("{host}:/{}", mount.container_path.join("/"));
    if !volume.persist() {
        argument.push_str(":O");
    }
    Ok(Bound { argument, volume })
}

/// `base` with `components` after it, joined by `/` as the tool
/// joins paths.
fn descend(base: &str, components: &[String]) -> String {
    let mut path = base.to_string();
    for component in components {
        path.push('/');
        path.push_str(component);
    }
    path
}

/// A host path as the tool and podman see it: on Linux, the path
/// itself.
#[cfg(target_os = "linux")]
pub(super) fn tool_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// A host path as the tool and podman see it: the machine's view of
/// it.
#[cfg(not(target_os = "linux"))]
pub(super) fn tool_path(path: &Path) -> String {
    crate::tools::podman::path(path)
}
