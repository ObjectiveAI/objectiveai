//! Bringing a container up, in order.

use std::sync::Arc;

use indexmap::IndexMap;

use super::content;
use super::encoded::encoded;
use super::family::Runs;
use super::own::Own;
use super::render;
use crate::container_proxy::fuse::mount;
use crate::container_proxy::requests;
use crate::server::container::Container as _;
use crate::server::container_client::ContainerClient;
use crate::server::container_deployer::ContainerDeployer;
use crate::server::content_store::ContentStore;
use crate::server::deployment::Deployment;
use crate::server::image_registry::ImageRegistry;
use crate::server::image_source::ImageSource;
use crate::server::mount::Mount;
use crate::server::scope_handle::ScopeHandle;
use crate::shared::containers::fuse::Kind;
use crate::shared::containers::request::{Container, FuseMount, Image};
use crate::shared::error::Error;

/// A container that is up: running, its proxy dialled, its asks
/// arriving.
pub(crate) struct Prepared<C> {
    /// The container, to stop when the run is over.
    pub container: C,
    /// The client every exchange with it goes through.
    pub client: ContainerClient,
    /// Its asks, from the one `/requests` connection.
    pub asks: requests::execute::ExecuteStream,
    /// The registry repository serving its image, to release when the
    /// run is over; `None` for an image the caller does not hold.
    pub repository: Option<String>,
    /// Every mount's path — volume, identity, FUSE — which every
    /// filetree opened on the container leaves out.
    pub ignore: Vec<Vec<String>>,
}

/// Everything before the id, in the only order that works:
///
/// 1. The content the caller mounts by identity, in the store — what
///    it lacks fetched from the caller now, on the scope's channels,
///    because a deploy binds it and cannot wait for it.
/// 2. The deployment, built: limits, volumes stamped with the caller,
///    identities. Every mount's path — volume, identity, FUSE — is
///    kept beside it, for every filetree opened on the container to
///    leave alone.
/// 3. For an image the caller holds, the registry told to serve a
///    repository from this scope — before the deploy, because the
///    deploy is what pulls it.
/// 4. The deploy.
/// 5. The proxy dialled: `/requests`, the one connection that carries
///    the container's asks. A proxy that does not answer is a
///    container that never came up.
/// 6. Every FUSE mount made, one request each on `/fuse/mount`, the
///    files then the directories, each answered only once it is
///    serving. A mount the proxy does not make is the run never
///    coming up.
///
/// The mounts are complete when this returns — which is before the
/// agent is registered and before the id, and so before any channel
/// the caller opened is read: a filetree asked for early waits in the
/// scope's inbox, as everything the caller opens does.
///
/// An [`Err`] is the run's `Error`, and whatever was made before the
/// failure is unmade: a container stopped, a repository released.
pub(crate) async fn prepare<R, D, S, G>(
    scope: &Arc<ScopeHandle>,
    client_identity: &str,
    request: &Container,
    deployer: &D,
    store: &S,
    registry: &G,
) -> Result<Prepared<D::Container>, Error>
where
    R: Runs,
    D: ContainerDeployer,
    D::Error: Into<Error>,
    S: ContentStore,
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
            deployer.server(client_identity, &deployment, name, digest).await.map_err(Into::into)?,
            None,
        ),
        Image::Registry { reference } => (
            deployer.registry(client_identity, &deployment, reference).await.map_err(Into::into)?,
            None,
        ),
    };

    let client = ContainerClient::new(container.address());
    let asks = match requests::execute::execute(&client).await {
        Ok(asks) => asks,
        Err(error) => {
            container.stop().await;
            if let Some(repository) = &repository {
                registry.release(repository).await;
            }
            return Err(render::proxy(error));
        }
    };

    if let Err(error) = mounts(&client, request).await {
        container.stop().await;
        if let Some(repository) = &repository {
            registry.release(repository).await;
        }
        return Err(error);
    }

    Ok(Prepared {
        container,
        client,
        asks,
        repository,
        ignore,
    })
}

/// Every FUSE mount, made in turn: the files, then the directories.
/// Each request returns once its mount is serving, so the last
/// returning is every mount complete.
async fn mounts(client: &ContainerClient, request: &Container) -> Result<(), Error> {
    let files = request.fuse_file_mounts.iter().map(|mount| (mount, Kind::File));
    let directories = request.fuse_directory_mounts.iter().map(|mount| (mount, Kind::Directory));
    for (mount, kind) in files.chain(directories) {
        let request = mount_request(mount, kind);
        mount::execute::execute(client, &request)
            .await
            .map_err(|error| render::mount_failed(&mount.id, error))?;
    }
    Ok(())
}

/// A FUSE mount of the request, as the proxy is asked for it.
fn mount_request(mount: &FuseMount, kind: Kind) -> mount::request::Request {
    mount::request::Request {
        path: mount.container_path.clone(),
        id: mount.id.clone(),
        readonly: mount.readonly,
        kind,
    }
}

/// The deployment a request asks for, plus what the provider adds.
fn deployment(client_identity: &str, request: &Container) -> Deployment {
    Deployment {
        memory: request.memory,
        disk: request.disk,
        // Nothing of the handler's: the proxy reads no environment
        // the handler sets. What a provider reserves for its own use
        // it adds in the deployer.
        environment: IndexMap::new(),
        mounts: request
            .volume_mounts
            .iter()
            .map(|mount| Mount {
                client_identity: client_identity.to_string(),
                host_name: mount.host_name.clone(),
                host_relative_path: mount.host_relative_path.clone(),
                container_path: mount.container_path.clone(),
            })
            .collect(),
        identity_file_mounts: request.identity_file_mounts.clone(),
        identity_directory_mounts: request.identity_directory_mounts.clone(),
    }
}

/// Every mount's path. Every mount is somebody else's tree: the
/// filetree reports the image's filesystem and what the container
/// makes of it.
fn ignored(request: &Container) -> Vec<Vec<String>> {
    request
        .volume_mounts
        .iter()
        .map(|mount| mount.container_path.clone())
        .chain(request.identity_file_mounts.iter().map(|mount| mount.container_path.clone()))
        .chain(request.identity_directory_mounts.iter().map(|mount| mount.container_path.clone()))
        .chain(request.fuse_file_mounts.iter().map(|mount| mount.container_path.clone()))
        .chain(request.fuse_directory_mounts.iter().map(|mount| mount.container_path.clone()))
        .collect()
}

/// The manifest ask, as the family spells it. A digest is a string
/// and a string always serializes.
fn manifest_ask<R: Runs>(digest: &str) -> Vec<u8> {
    encoded(&R::Ask::from(Own::OciManifest(digest))).unwrap_or_default()
}

/// The blob ask, likewise.
fn blob_ask<R: Runs>(digest: &str) -> Vec<u8> {
    encoded(&R::Ask::from(Own::OciBlob(digest))).unwrap_or_default()
}
