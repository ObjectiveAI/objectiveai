//! Bringing a container up, in order.

use std::sync::Arc;

use indexmap::IndexMap;

use super::content;
use super::encoded::encoded;
use super::family::Runs;
use super::own::Own;
use super::render;
use crate::container_proxy::filesystem;
use crate::container_proxy::filesystem::tree::{IGNORE_ENV, Ignore};
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
use crate::shared::containers::request::{Container, Image};
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
}

/// Everything before the id, in the only order that works:
///
/// 1. The content the caller mounts by identity, in the store — what
///    it lacks fetched from the caller now, on the scope's channels,
///    because a deploy binds it and cannot wait for it.
/// 2. The deployment, built: limits, volumes stamped with the caller,
///    identities, and the environment the proxy reads — its FUSE
///    mounts, and every mount's path for the filetree to leave alone.
/// 3. For an image the caller holds, the registry told to serve a
///    repository from this scope — before the deploy, because the
///    deploy is what pulls it.
/// 4. The deploy.
/// 5. The proxy dialled: `/requests`, the one connection that carries
///    the container's asks. A proxy that does not answer is a
///    container that never came up.
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
    match requests::execute::execute(&client).await {
        Ok(asks) => Ok(Prepared {
            container,
            client,
            asks,
            repository,
        }),
        Err(error) => {
            container.stop().await;
            if let Some(repository) = &repository {
                registry.release(repository).await;
            }
            Err(render::proxy(error))
        }
    }
}

/// The deployment a request asks for, plus what the provider adds.
fn deployment(client_identity: &str, request: &Container) -> Deployment {
    let mut environment = IndexMap::new();
    let mounts = filesystem::Mounts {
        files: request.fuse_file_mounts.iter().map(fuse_mount).collect(),
        directories: request.fuse_directory_mounts.iter().map(fuse_mount).collect(),
    };
    environment.insert(filesystem::MOUNTS_ENV.to_string(), mounts.to_string());
    // Every mount is somebody else's tree: the filetree reports the
    // image's filesystem and what the container makes of it.
    let ignore = Ignore(
        request
            .volume_mounts
            .iter()
            .map(|mount| mount.container_path.clone())
            .chain(request.identity_file_mounts.iter().map(|mount| mount.container_path.clone()))
            .chain(request.identity_directory_mounts.iter().map(|mount| mount.container_path.clone()))
            .chain(request.fuse_file_mounts.iter().map(|mount| mount.container_path.clone()))
            .chain(request.fuse_directory_mounts.iter().map(|mount| mount.container_path.clone()))
            .collect(),
    );
    environment.insert(IGNORE_ENV.to_string(), ignore.to_string());
    Deployment {
        memory: request.memory,
        disk: request.disk,
        environment,
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

/// A FUSE mount as the proxy reads it.
fn fuse_mount(mount: &crate::shared::containers::request::FuseMount) -> filesystem::Mount {
    filesystem::Mount {
        path: mount.container_path.clone(),
        id: mount.id.clone(),
        readonly: mount.readonly,
    }
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
