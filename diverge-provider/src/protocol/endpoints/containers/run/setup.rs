//! Bringing a container up, in order.

use diverge_sdk::shared::Identity;
use std::sync::Arc;

use futures_util::future;
use indexmap::IndexMap;
use serde_json::Value;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

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
use diverge_sdk::provider::endpoints::containers::client::UnaryError;
use diverge_sdk::wire::decode::Decode as _;
use diverge_sdk::wire::server::answer::{Answer, answer};
use crate::protocol::caller::Caller;
use crate::protocol::container::{self, Container as _};
use crate::protocol::release::Release;
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
use diverge_sdk::shared::containers::registration;
use diverge_sdk::shared::containers::secrets::Values;
use diverge_sdk::shared::error::Error;

/// A container brought up and ready to serve.
pub(crate) struct Prepared {
    /// The id, minted here and entered in the directory before the
    /// registration was asked for: what the run answers.
    pub id: String,
    /// The directory entry under [`id`](Self::id) taken out: the
    /// teardown's first step, and the drop's if it never gets there.
    pub removal: Release,
    /// The container stopped and then the task routing its proxy's
    /// connection aborted: the teardown's second step, and the drop's
    /// if it never gets there.
    pub stop: Release,
    /// The one connection to its proxy.
    pub proxy: Handle,
    /// The begin scope on it, and what rides it.
    pub begun: Begun,
    /// Every FUSE mount, made, with the asks each will make.
    pub mounts: Vec<Mount>,
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
/// 2. The container is deployed, its proxy listening, and the
///    repository is released as the deploy answers, whichever way: an
///    image deployed is local, and nothing pulls through the
///    repository again, so it is held for the deploy and not for the
///    run.
/// 3. The proxy is dialled: one WebSocket, for the container's life,
///    its router's task kept to abort once the container is stopped.
/// 4. The family's `begin` is opened on it — carrying the arguments
///    and the template — and its `Begun` awaited, with what the
///    container declared: the dependencies, for an agent container,
///    and the secrets it takes, by name and mode. No value of any
///    secret is in the begin.
/// 5. The id is minted and the container entered in the directory
///    under it — with its proxy connection and its begin scope — so
///    that a `containers::fuse_serve` naming the id is answered from now,
///    which is what lets the caller serve the container's own paths
///    into its dependencies. The id is not answered yet.
/// 6. Beside each other: the container is registered with the caller
///    — the family's [`register`](Runs::register), one ask carrying
///    the dependencies to deploy and the secrets to answer, the id
///    told with them, nothing asked when it declared neither — and
///    the values the caller answered are fed to the container on the
///    begin's secrets channel, awaited until the program has applied
///    them, the program's refusal the run's error; and one
///    `fuse::mount` scope per mount is opened, file mounts first,
///    each answered before the next. Both are awaited to their end
///    before either's failure is acted on.
///
/// Every failure after the deploy takes the container out of the
/// directory, when it was entered, and stops it; the error is the
/// run's. Nothing the caller opened has been read yet, and the id is
/// not out.
///
/// # Every release is a [`Release`]
///
/// The repository, the container and the directory entry are each
/// given back by a [`Release`] made as it is taken: run in place on
/// every failure above and by the run's teardown, and spawned by its
/// drop when a panic or a cancel means neither gets there. So nothing
/// taken here outlives its run, however the run ends.
pub(crate) async fn prepare<R, D, G, L>(
    scope: &Arc<ScopeHandle>,
    client_identity: &Arc<Identity>,
    request: &Container,
    arguments: Value,
    deployer: &D,
    registry: &Arc<G>,
    held: &Held<L>,
    directory: &Arc<Directory>,
) -> Result<Prepared, Error>
where
    R: Runs,
    D: ContainerDeployer,
    D::Error: Into<Error>,
    G: ImageRegistry + 'static,
    G::Error: Into<Error>,
    L: Volume + 'static,
    L::Error: Into<Error>,
{
    let deployment = deployment(client_identity, request);
    let watched: Arc<[Watched]> = Arc::from(watched(request, held.volumes()));

    let repository = uuid::Uuid::new_v4().to_string();
    let source = ImageSource::new(Arc::clone(scope), manifest_ask::<R>, blob_ask::<R>);
    registry.serve(&repository, source).await.map_err(Into::into)?;
    let released = Release::new({
        let registry = Arc::clone(registry);
        let repository = repository.clone();
        async move { registry.release(&repository).await }
    });
    let caller = Caller::new(
        Arc::clone(scope),
        has_ask::<R>,
        request.image.digest.clone(),
        registry.address(),
        repository,
    );
    let deployed = deployer
        .deploy(client_identity, &deployment, &request.image, &caller)
        .await;
    released.now().await;
    let container = Arc::new(deployed.map_err(Into::into)?);
    let (route, routed) = oneshot::channel();
    let stop = stopping(Arc::clone(&container), routed);

    let proxy = match proxy::dial(container.address()).await {
        Ok((proxy, router)) => {
            let _ = route.send(router);
            proxy
        }
        Err(error) => {
            stop.now().await;
            return Err(render::proxy(error));
        }
    };

    let begun = match R::begin(&proxy, arguments, request.template.clone()).await {
        Ok(begun) => begun,
        Err(error) => {
            stop.now().await;
            return Err(error);
        }
    };

    let id = Directory::mint();
    directory.insert(id.clone(), Arc::clone(client_identity), proxy.clone()).await;
    let removal = Release::new({
        let directory = Arc::clone(directory);
        let id = id.clone();
        async move { directory.remove(&id).await }
    });

    let (fed, mounts) = future::join(fed::<R>(scope, &id, &begun), mounts(&proxy, request)).await;
    let mounts = match (fed, mounts) {
        (Ok(()), Ok(mounts)) => mounts,
        (Err(error), _) | (Ok(()), Err(error)) => {
            removal.now().await;
            stop.now().await;
            return Err(error);
        }
    };

    Ok(Prepared {
        id,
        removal,
        stop,
        proxy,
        begun,
        mounts,
        watched,
    })
}

/// The container's [`Release`]: the container stopped, and then the
/// task routing its proxy's connection aborted, when the dial got as
/// far as making one — which `routed` says, holding the task once the
/// dial made it and nothing when the dial failed or never answered.
fn stopping<C>(container: Arc<C>, mut routed: oneshot::Receiver<JoinHandle<()>>) -> Release
where
    C: container::Container + 'static,
{
    Release::new(async move {
        container.stop().await;
        if let Ok(router) = routed.try_recv() {
            router.abort();
        }
    })
}

/// The container registered with the caller and fed its first
/// values, in order: the family's [`register`](Runs::register), then
/// — when it asked anything — the values the caller answered fed on
/// the begin's secrets channel, awaited until the program has applied
/// them. The program's refusal of the values is the run's error in
/// the program's words; the proxy failing to answer is the run's
/// error too. Nothing is delivered to the container by its proxy
/// until this feed, which is why it precedes the id.
async fn fed<R: Runs>(scope: &ScopeHandle, id: &str, begun: &Begun) -> Result<(), Error> {
    let Some(values) = R::register(scope, id, &begun.dependencies, &begun.secrets, &begun.permissions).await? else {
        return Ok(());
    };
    match begun.begin.secrets(values).await {
        Ok(()) => Ok(()),
        Err(UnaryError::Refused(error)) => Err(error),
        Err(error) => Err(render::proxy(error)),
    }
}

/// The caller's answer to a registration ask, read to the finish so
/// its number comes back to the run: one frame back — the values, the
/// registration accepted; the caller's refusal is the run's error in
/// the caller's words; a finish with nothing, or an answer this end
/// cannot read, is the run's error too. Either family opens the ask
/// when its container declared anything.
pub(crate) async fn registered(scope: &ScopeHandle, payload: &[u8]) -> Result<Values, Error> {
    let mut channel = scope.send_channel_request(payload).await;
    let mut outcome = Err(render::registration_unserved());
    while let Some(bytes) = channel.response_receiver.recv().await {
        match answer(&bytes) {
            Some(Answer::Frame(payload)) => {
                outcome = match registration::response::Frame::decode(&payload) {
                    Ok(registration::response::Frame::Deployed(values)) => Ok(values),
                    Ok(registration::response::Frame::Error(error)) => Err(error),
                    Err(error) => Err(render::registration_failed(error)),
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
fn deployment(client_identity: &Identity, request: &Container) -> Deployment {
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
                client_identity: client_identity.clone(),
                volume_name: mount.volume_name.clone(),
                volume_relative_path: mount.volume_relative_path.clone(),
                container_path: mount.container_path.clone(),
            })
            .collect(),
    }
}

/// What is watched beside the proxy's tree: the mount of every volume
/// the provider watches itself, which [`Volume::tree`] says, by the
/// volume at the mount's relative path, re-rooted at the mount's
/// container path when a filetree merges it in. The proxy's tree
/// leaves each of those mounts out, and every FUSE mount — see
/// [`Run::ignored`](super::run::Run::ignored). Every other volume
/// mount is content the proxy walks and watches, and seeing it change
/// is what a filetree is for. `held` holds the volumes in the order
/// the request names them.
fn watched<L>(request: &Container, held: &[Arc<L>]) -> Vec<Watched>
where
    L: Volume + 'static,
    L::Error: Into<Error>,
{
    let mut watched = Vec::new();
    for (mount, volume) in request.volume_mounts.iter().zip(held) {
        if volume.tree() {
            continue;
        }
        watched.push(Watched {
            container_path: mount.container_path.clone(),
            watcher: Arc::new(Watching {
                volume: Arc::clone(volume),
                path: mount.volume_relative_path.clone(),
            }),
        });
    }
    watched
}

/// The ask whether the caller holds the image, as this family's
/// frame.
fn has_ask<R: Runs>(digest: &str) -> Vec<u8> {
    R::Ask::try_from(Own::OciHas(digest)).ok().and_then(|ask| encoded(&ask)).unwrap_or_default()
}

/// The ask for a manifest the registry does not hold, as this
/// family's frame.
fn manifest_ask<R: Runs>(digest: &str) -> Vec<u8> {
    R::Ask::try_from(Own::OciManifest(digest)).ok().and_then(|ask| encoded(&ask)).unwrap_or_default()
}

/// The ask for a blob the registry does not hold, as this family's
/// frame.
fn blob_ask<R: Runs>(digest: &str) -> Vec<u8> {
    R::Ask::try_from(Own::OciBlob(digest)).ok().and_then(|ask| encoded(&ask)).unwrap_or_default()
}
