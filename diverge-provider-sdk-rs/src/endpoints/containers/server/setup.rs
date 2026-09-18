//! Bringing a container up, in order.

use std::sync::Arc;

use indexmap::IndexMap;
use serde_json::Value;

use super::begin::Begun;
use super::encoded::encoded;
use super::family::Runs;
use super::held::Held;
use super::own::Own;
use super::render;
use crate::client::handle::Handle;
use crate::container_proxy_endpoints::client::Asks;
use crate::container_proxy_endpoints::fuse::mount::client::execute::{self as mount, Ask as MountAsk};
use crate::server::caller::Caller;
use crate::server::container::{self, Container as _};
use crate::server::container_deployer::ContainerDeployer;
use crate::server::deployment::Deployment;
use crate::server::image_registry::ImageRegistry;
use crate::server::image_source::ImageSource;
use crate::server::mount::Mount as DeployedMount;
use crate::server::proxy;
use crate::server::scope_handle::ScopeHandle;
use crate::server::volume::Volume;
use crate::shared::containers::fuse::Kind;
use crate::shared::containers::request::{Container, FuseMount};
use crate::shared::error::Error;

/// A container brought up and ready to serve.
pub(crate) struct Prepared<C> {
    /// The container, to stop when the run is over.
    pub container: C,
    /// The one connection to its proxy.
    pub proxy: Handle,
    /// The begin scope on it, and what rides it.
    pub begun: Begun,
    /// Every FUSE mount, made, with the asks each will make.
    pub mounts: Vec<Mount>,
    /// The registry repository serving the caller's manifests and
    /// blobs for this run, to release.
    pub repository: String,
    /// Every path a filetree leaves out: every FUSE mount's, and
    /// every mount's of a volume whose listing says it is not in the
    /// tree.
    pub ignore: Vec<Vec<String>>,
}

/// One FUSE mount, made: the caller's id for it, the scope it lives
/// on, and the asks it makes there.
pub(crate) struct Mount {
    /// The caller's id, put back on every ask relayed.
    pub id: String,
    /// The scope, to answer the asks on.
    pub handle: mount::ExecuteHandle,
    /// The asks, as the mount makes them.
    pub asks: Asks<MountAsk>,
}

/// Bring the container up, in the order the specification states.
///
/// 1. The registry is told to serve the caller's manifests and blobs
///    under a fresh repository, and the deployer is asked for the
///    image by name and digest, with the caller's help at hand —
///    whether the caller holds it, and where the registry serves it.
///    Where the deployer gets the image is its own.
/// 2. The container is deployed, its proxy listening.
/// 3. The proxy is dialled: one WebSocket, for the container's life.
/// 4. The family's `begin` is opened on it — an agent container's
///    carrying the agent — and its `Begun` awaited.
/// 5. One `fuse::mount` scope per mount, file mounts first, each
///    answered before the next is opened.
///
/// Every failure after the deploy stops the container and releases
/// the repository; the error is the run's. Nothing the caller opened
/// has been read yet, and the id is not out.
pub(crate) async fn prepare<R, D, G, L>(
    scope: &Arc<ScopeHandle>,
    client_identity: &str,
    request: &Container,
    agent: Option<Value>,
    deployer: &D,
    registry: &G,
    held: &Held<L>,
) -> Result<Prepared<D::Container>, Error>
where
    R: Runs,
    D: ContainerDeployer,
    D::Error: Into<Error>,
    G: ImageRegistry,
    G::Error: Into<Error>,
    L: Volume,
{
    let deployment = deployment(client_identity, request);
    let ignore = ignored(request, held.volumes());

    let repository = uuid::Uuid::new_v4().to_string();
    let source = ImageSource::new(Arc::clone(scope), manifest_ask::<R>, blob_ask::<R>);
    registry.serve(&repository, source).await.map_err(Into::into)?;
    let caller = Caller::new(
        Arc::clone(scope),
        has_ask::<R>,
        request.image.name.clone(),
        request.image.digest.clone(),
        registry.address(),
        repository.clone(),
    );
    let container = match deployer
        .deploy(client_identity, &deployment, &request.image.name, &request.image.digest, &caller)
        .await
    {
        Ok(container) => container,
        Err(error) => {
            registry.release(&repository).await;
            return Err(error.into());
        }
    };

    let proxy = match proxy::dial(container.address()).await {
        Ok(proxy) => proxy,
        Err(error) => {
            undo(&container, &repository, registry).await;
            return Err(render::proxy(error));
        }
    };

    let begun = match R::begin(&proxy, agent).await {
        Ok(begun) => begun,
        Err(error) => {
            undo(&container, &repository, registry).await;
            return Err(error);
        }
    };

    let mounts = match mounts(&proxy, request).await {
        Ok(mounts) => mounts,
        Err(error) => {
            undo(&container, &repository, registry).await;
            return Err(error);
        }
    };

    Ok(Prepared {
        container,
        proxy,
        begun,
        mounts,
        repository,
        ignore,
    })
}

/// A failure after the deploy: the container stopped, the repository
/// released.
async fn undo<C, G>(container: &C, repository: &str, registry: &G)
where
    C: container::Container,
    G: ImageRegistry,
{
    container.stop().await;
    registry.release(repository).await;
}

/// Every FUSE mount, made in order: the file mounts, then the
/// directory mounts, each complete before the next is opened. A mount
/// the proxy does not make is the run's error.
async fn mounts(proxy: &Handle, request: &Container) -> Result<Vec<Mount>, Error> {
    let mut mounts = Vec::with_capacity(request.fuse_file_mounts.len() + request.fuse_directory_mounts.len());
    for (fuse_mount, kind) in request
        .fuse_file_mounts
        .iter()
        .map(|mount| (mount, Kind::File))
        .chain(request.fuse_directory_mounts.iter().map(|mount| (mount, Kind::Directory)))
    {
        mounts.push(one_mount(proxy, fuse_mount, kind).await?);
    }
    Ok(mounts)
}

/// One mount, complete.
async fn one_mount(proxy: &Handle, fuse_mount: &FuseMount, kind: Kind) -> Result<Mount, Error> {
    match mount::execute(proxy, fuse_mount.container_path.clone(), kind).await {
        Ok((handle, asks)) => Ok(Mount {
            id: fuse_mount.id.clone(),
            handle,
            asks,
        }),
        Err(error) => Err(render::mount_failed(&fuse_mount.id, error)),
    }
}

/// The deployment, from the request: what the deployer is handed.
fn deployment(client_identity: &str, request: &Container) -> Deployment {
    Deployment {
        memory: request.memory,
        disk: request.disk,
        // Nothing of the handler's: the proxy reads no environment
        // the handler sets.
        environment: IndexMap::new(),
        mounts: request
            .volume_mounts
            .iter()
            .map(|mount| DeployedMount {
                client_identity: client_identity.to_string(),
                host_name: mount.host_name.clone(),
                host_relative_path: mount.host_relative_path.clone(),
                container_path: mount.container_path.clone(),
                persist: mount.persist,
            })
            .collect(),
    }
}

/// What a filetree of this container leaves out: every FUSE mount's
/// path, and the path of every mount of a volume the provider keeps
/// out of the tree. A FUSE mount is the caller's own answers, and a
/// tree over it would report them back to the caller. A volume mount
/// is content on the provider, and seeing it change is what a
/// filetree is for — unless the provider said in the listing, and
/// says again through [`Volume::tree`], that this volume is not
/// walked, a dataset too large or too still to watch. `held` holds
/// the volumes in the order the request names them.
fn ignored<L: Volume>(request: &Container, held: &[L]) -> Vec<Vec<String>> {
    request
        .fuse_file_mounts
        .iter()
        .map(|mount| mount.container_path.clone())
        .chain(request.fuse_directory_mounts.iter().map(|mount| mount.container_path.clone()))
        .chain(
            request
                .volume_mounts
                .iter()
                .zip(held)
                .filter(|(_, volume)| !volume.tree())
                .map(|(mount, _)| mount.container_path.clone()),
        )
        .collect()
}

/// The ask whether the caller holds the image, as this family's
/// frame.
fn has_ask<R: Runs>(name: &str, digest: &str) -> Vec<u8> {
    encoded(&R::Ask::from(Own::OciHas { name, digest })).unwrap_or_default()
}

/// The ask for a manifest the registry does not hold, as this
/// family's frame.
fn manifest_ask<R: Runs>(digest: &str) -> Vec<u8> {
    encoded(&R::Ask::from(Own::OciManifest(digest))).unwrap_or_default()
}

/// The ask for a blob the registry does not hold, as this family's
/// frame.
fn blob_ask<R: Runs>(digest: &str) -> Vec<u8> {
    encoded(&R::Ask::from(Own::OciBlob(digest))).unwrap_or_default()
}
