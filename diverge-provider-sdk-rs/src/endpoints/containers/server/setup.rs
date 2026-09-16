//! Bringing a container up, in order.

use std::sync::Arc;

use indexmap::IndexMap;
use serde_json::Value;

use super::begin::Begun;
use super::content;
use super::encoded::encoded;
use super::family::Runs;
use super::own::Own;
use super::render;
use crate::client::handle::Handle;
use crate::container_proxy_endpoints::client::Asks;
use crate::container_proxy_endpoints::fuse::mount::client::execute::{self as mount, Ask as MountAsk};
use crate::server::container::{self, Container as _};
use crate::server::container_deployer::ContainerDeployer;
use crate::server::identity_mount_manager::IdentityMountManager;
use crate::server::deployment::Deployment;
use crate::server::image_registry::ImageRegistry;
use crate::server::image_source::ImageSource;
use crate::server::mount::Mount as DeployedMount;
use crate::server::proxy;
use crate::server::scope_handle::ScopeHandle;
use crate::shared::containers::fuse::Kind;
use crate::shared::containers::request::{Container, FuseMount, Image};
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
    /// The registry repository serving the image, to release; [`None`]
    /// unless the image is the caller's.
    pub repository: Option<String>,
    /// Every FUSE mount's path, which a filetree leaves out; a volume
    /// or an identity mount is in the tree.
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
/// 1. The content every identity mount names is held — fetched from
///    the caller where the store lacks it, on channels this end
///    opens.
/// 2. A caller-held image is put on the provider's registry, fed by
///    digest from the caller.
/// 3. The container is deployed, its proxy listening.
/// 4. The proxy is dialled: one WebSocket, for the container's life.
/// 5. The family's `begin` is opened on it — an agent container's
///    carrying the agent — and its `Begun` awaited.
/// 6. One `fuse::mount` scope per mount, file mounts first, each
///    answered before the next is opened.
///
/// Every failure after the deploy stops the container and releases
/// the repository; the error is the run's. Nothing the caller opened
/// has been read yet, and the id is not out.
pub(crate) async fn prepare<R, D, S, G>(
    scope: &Arc<ScopeHandle>,
    client_identity: &str,
    request: &Container,
    agent: Option<Value>,
    deployer: &D,
    store: &S,
    registry: &G,
) -> Result<Prepared<D::Container>, Error>
where
    R: Runs,
    D: ContainerDeployer,
    D::Error: Into<Error>,
    S: IdentityMountManager,
    S::Error: Into<Error>,
    G: ImageRegistry,
    G::Error: Into<Error>,
{
    content::ensure::<R, S>(scope, store, request).await?;
    let deployment = deployment(client_identity, request);
    let ignore = ignored(request);

    let (container, repository) = match &request.image {
        Image::Client { name, digest } => {
            let repository = uuid::Uuid::new_v4().to_string();
            let source = ImageSource::new(Arc::clone(scope), manifest_ask::<R>, blob_ask::<R>);
            registry.serve(&repository, source).await.map_err(Into::into)?;
            match deployer
                .client(client_identity, &deployment, name, digest, registry.address(), &repository)
                .await
            {
                Ok(container) => (container, Some(repository)),
                Err(error) => {
                    registry.release(&repository).await;
                    return Err(error.into());
                }
            }
        }
        Image::Server { name, digest } => (
            deployer
                .server(client_identity, &deployment, name, digest)
                .await
                .map_err(Into::into)?,
            None,
        ),
        Image::Registry { reference } => (
            deployer
                .registry(client_identity, &deployment, reference)
                .await
                .map_err(Into::into)?,
            None,
        ),
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
async fn undo<C, G>(container: &C, repository: &Option<String>, registry: &G)
where
    C: container::Container,
    G: ImageRegistry,
{
    container.stop().await;
    if let Some(repository) = repository {
        registry.release(repository).await;
    }
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
        identity_file_mounts: request.identity_file_mounts.clone(),
        identity_directory_mounts: request.identity_directory_mounts.clone(),
    }
}

/// Every FUSE mount's path: what a filetree of this container leaves
/// out. A FUSE mount is the caller's own answers, and a tree over it
/// would report them back to the caller; a volume or an identity
/// mount is content on the provider, and seeing it change is what a
/// filetree is for, so neither is listed.
fn ignored(request: &Container) -> Vec<Vec<String>> {
    request
        .fuse_file_mounts
        .iter()
        .map(|mount| mount.container_path.clone())
        .chain(request.fuse_directory_mounts.iter().map(|mount| mount.container_path.clone()))
        .collect()
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
