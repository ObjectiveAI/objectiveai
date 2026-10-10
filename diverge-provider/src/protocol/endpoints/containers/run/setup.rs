//! Bringing a container up, in order.

use std::sync::Arc;

use futures_util::future;
use indexmap::IndexMap;
use serde_json::Value;

use super::begin::Begun;
use super::encoded::encoded;
use super::family::Runs;
use super::own::Own;
use super::held::Held;
use super::render;
use super::watched::{Watched, Watching};
use diverge_sdk::wire::client::handle::Handle;
use diverge_sdk::container_proxy::outside::client::Asks;
use diverge_sdk::container_proxy::outside::endpoints::fuse::mount::client::execute::{self as mount, Ask as MountAsk};
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::server::answer::{Answer, answer};
use crate::protocol::caller::Caller;
use crate::protocol::container::{self, Container as _};
use crate::protocol::container_deployer::ContainerDeployer;
use crate::protocol::deployment::Deployment;
use crate::protocol::directory::Directory;
use crate::protocol::image_registry::ImageRegistry;
use crate::protocol::image_source::ImageSource;
use crate::protocol::mount::Mount as DeployedMount;
use crate::protocol::proxy;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use crate::protocol::volume::Volume;
use diverge_sdk::shared::containers::fuse::Kind;
use diverge_sdk::shared::containers::request::{Container, FuseMount};
use diverge_sdk::shared::containers::dependencies;
use diverge_sdk::shared::error::Error;

/// A container brought up and ready to serve.
pub(crate) struct Prepared<C> {
    /// The id, minted here and entered in the directory before the
    /// dependencies were asked for: what the run answers.
    pub id: String,
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
    /// Every path the proxy's tree leaves out: every FUSE mount's,
    /// and every mount's of a volume the provider watches itself.
    pub ignore: Vec<Vec<String>>,
    /// Every mount of a volume the provider watches itself, with the
    /// way to, for a filetree to merge in.
    pub watched: Arc<[Watched]>,
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
///    image, with the caller's help at hand — whether the caller holds
///    it, under what path, and where the registry serves it.
///    Where the deployer gets the image is its own.
/// 2. The container is deployed, its proxy listening.
/// 3. The proxy is dialled: one WebSocket, for the container's life.
/// 4. The family's `begin` is opened on it — carrying the arguments
///    — and its `Begun` awaited, with the dependencies an agent
///    container declared.
/// 5. The id is minted and the container entered in the directory
///    under it — with its proxy connection and its begin scope — so
///    that a `containers::serve` naming the id is answered from now,
///    which is what lets the caller serve the container's own paths
///    into its dependencies. The id is not answered yet.
/// 6. Beside each other: the caller is asked to deploy those
///    dependencies, when there are any, the id told with them — the
///    agents family's [`deploy`](Runs::deploy); a tool container
///    declares nothing — and one `fuse::mount`
///    scope per mount is opened, file mounts first, each answered
///    before the next. Both are awaited to their end before either's
///    failure is acted on.
///
/// Every failure after the deploy stops the container, takes it out
/// of the directory and releases the repository; the error is the
/// run's. Nothing the caller opened has been read yet, and the id is
/// not out.
pub(crate) async fn prepare<R, D, G, L>(
    scope: &Arc<ScopeHandle>,
    client_identity: &Arc<str>,
    request: &Container,
    arguments: Value,
    deployer: &D,
    registry: &G,
    held: &Held<L>,
    directory: &Directory,
) -> Result<Prepared<D::Container>, Error>
where
    R: Runs,
    D: ContainerDeployer,
    D::Error: Into<Error>,
    G: ImageRegistry,
    G::Error: Into<Error>,
    L: Volume + 'static,
    L::Error: Into<Error>,
{
    let deployment = deployment(client_identity, request);
    let (ignore, watched) = excluded(request, held.volumes());
    let watched: Arc<[Watched]> = Arc::from(watched);

    let repository = uuid::Uuid::new_v4().to_string();
    let source = ImageSource::new(Arc::clone(scope), manifest_ask::<R>, blob_ask::<R>);
    registry.serve(&repository, source).await.map_err(Into::into)?;
    let caller = Caller::new(
        Arc::clone(scope),
        has_ask::<R>,
        request.image.digest.clone(),
        registry.address(),
        repository.clone(),
    );
    let container = match deployer
        .deploy(client_identity, &deployment, &request.image, &caller)
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

    let begun = match R::begin(&proxy, arguments).await {
        Ok(begun) => begun,
        Err(error) => {
            undo(&container, &repository, registry).await;
            return Err(error);
        }
    };

    let id = Directory::mint();
    directory.insert(id.clone(), Arc::clone(client_identity), proxy.clone()).await;

    let (deployed, mounts) = future::join(R::deploy(scope, &id, &begun.dependencies), mounts(&proxy, request)).await;
    let mounts = match (deployed, mounts) {
        (Ok(()), Ok(mounts)) => mounts,
        (Err(error), _) | (Ok(()), Err(error)) => {
            directory.remove(&id).await;
            undo(&container, &repository, registry).await;
            return Err(error);
        }
    };

    Ok(Prepared {
        id,
        container,
        proxy,
        begun,
        mounts,
        repository,
        ignore,
        watched,
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

/// The caller's answer to a dependencies ask, read to the finish so its
/// number comes back to the run: one frame back — a deploy is `Ok`;
/// the caller's refusal is the run's error in the caller's words; a
/// finish with nothing, or an answer this end cannot read, is the
/// run's error too. The agents family opens the ask; a tool container
/// declares nothing and opens none.
pub(crate) async fn deployed(scope: &ScopeHandle, payload: &[u8]) -> Result<(), Error> {
    let mut channel = scope.send_channel_request(payload).await;
    let mut outcome = Err(render::dependencies_unserved());
    while let Some(bytes) = channel.response_receiver.recv().await {
        match answer(&bytes) {
            Some(Answer::Frame(payload)) => {
                outcome = match dependencies::response::Frame::decode(&payload) {
                    Ok(dependencies::response::Frame::Deployed) => Ok(()),
                    Ok(dependencies::response::Frame::Error(error)) => Err(error),
                    Err(error) => Err(render::dependencies_failed(error)),
                };
            }
            Some(Answer::Finish) => break,
            None => {}
        }
    }
    outcome
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
                volume_name: mount.volume_name.clone(),
                volume_relative_path: mount.volume_relative_path.clone(),
                container_path: mount.container_path.clone(),
            })
            .collect(),
    }
}

/// What the proxy's tree leaves out, and what is watched instead.
/// Left out: every FUSE mount's path, since a FUSE mount is the
/// caller's own answers and a tree over it would report them back to
/// the caller; and the path of every mount of a volume the provider
/// watches itself, which [`Volume::tree`] says. Watched instead: each
/// of those volume mounts, by the volume at the mount's relative
/// path, re-rooted at the mount's container path when a filetree
/// merges it in. Every other volume mount is content the proxy walks
/// and watches, and seeing it change is what a filetree is for.
/// `held` holds the volumes in the order the request names them.
fn excluded<L>(request: &Container, held: &[Arc<L>]) -> (Vec<Vec<String>>, Vec<Watched>)
where
    L: Volume + 'static,
    L::Error: Into<Error>,
{
    let mut ignore: Vec<Vec<String>> = request
        .fuse_file_mounts
        .iter()
        .map(|mount| mount.container_path.clone())
        .chain(request.fuse_directory_mounts.iter().map(|mount| mount.container_path.clone()))
        .collect();
    let mut watched = Vec::new();
    for (mount, volume) in request.volume_mounts.iter().zip(held) {
        if volume.tree() {
            continue;
        }
        ignore.push(mount.container_path.clone());
        watched.push(Watched {
            container_path: mount.container_path.clone(),
            watcher: Arc::new(Watching {
                volume: Arc::clone(volume),
                path: mount.volume_relative_path.clone(),
            }),
        });
    }
    (ignore, watched)
}

/// The ask whether the caller holds the image, as this family's
/// frame.
fn has_ask<R: Runs>(digest: &str) -> Vec<u8> {
    encoded(&R::Ask::from(Own::OciHas(digest))).unwrap_or_default()
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
