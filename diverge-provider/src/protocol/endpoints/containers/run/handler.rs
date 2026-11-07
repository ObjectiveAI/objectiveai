//! One run, from its request to its finish.

use diverge_sdk::shared::Identity;
use std::sync::Arc;

use serde_json::Value;

use super::begin::Begun;
use super::check::check;
use super::family::Runs;
use super::held::{Held, Refused};
use super::ledger::Ledger;
use super::run::{Run, send};
use super::{relay, serve, setup};
use crate::protocol::container_deployer::ContainerDeployer;
use crate::protocol::directory::Directory;
use crate::protocol::image_registry::ImageRegistry;
use crate::protocol::release::Release;
use diverge_sdk::wire::server::scope_handle::ScopeHandle;
use crate::protocol::volume_manager::VolumeManager;
use diverge_sdk::shared::containers::request::Container;
use diverge_sdk::provider::endpoints::info::server::response::Capabilities;
use diverge_sdk::shared::containers::response::{Id, VolumeFuseServed, VolumeHeld, VolumeMountUnsupported, VolumeMounted};
use diverge_sdk::shared::error::Error;

/// Run the container the request describes for as long as the scope
/// lives, and end the scope.
///
/// In order:
///
/// 0. A provider that runs no containers — its `containers`
///    capability `false`, see [`Capabilities`] — refuses the run
///    with the family's `Unsupported`, and the scope finishes on it:
///    before the mounts are checked, before anything is held, fetched
///    or deployed.
/// 1. The request's mounts are checked — see [`check`] — or the run
///    is refused with the error, before anything is held. Then every volume the
///    request names is found and held, shared — see [`Held`] — or
///    the run is refused: with the name of a volume held, mounted,
///    FUSE served or of a mode the provider does not mount, with the
///    mode of one not in the mode its mount means, or with an error
///    for a name that is not the caller's. Before anything is fetched
///    or deployed. The holds are the `Held`'s and are given back at
///    every ending below.
/// 2. The container is brought up — see
///    [`setup::prepare`](super::setup::prepare): registry,
///    deploy, the proxy dialled, the family's begin, the id minted
///    and the container entered in the directory — with its proxy
///    connection, for serves and transfers — then the container
///    registered with the caller, the id told, its first secrets fed
///    back to it, beside every mount. A failure is the run's error,
///    and the scope finishes on it.
/// 3. The id is sent. From here the container is running for the
///    caller.
/// 4. The relays are spawned: the proxy's asks on the begin scope,
///    each mount's asks, and — on an agent container — the agent's
///    chunks onto the main stream, the loop's `Active` and
///    `Inactive` among them; on a tool container, its `Active` and
///    `Inactive` onto the main stream, the begin's end heard with
///    them.
/// 5. The caller's channels are served until the run ends: a stop,
///    the container leaving, or the caller going away. Among them a
///    `fuse_mount`, one more FUSE mount made while the container
///    runs, checked against every mount it holds — see
///    [`Ledger`] — and answered once the proxy has.
/// 6. The teardown, the same for every ending: the directory entry
///    removed, the container stopped and its proxy's router ended, the
///    volumes given back, every task ended, and the finish. The
///    repository is not among them: it was released as the deploy
///    answered. Every step of it is a release that also runs at the
///    drop — see [`Release`](crate::protocol::release::Release) and
///    [`Held`] — so a run that panics or is cancelled before here still
///    gives everything back.
pub(crate) async fn run<R, D, G, V>(
    scope: ScopeHandle,
    client_identity: &Identity,
    request: &Container,
    arguments: Value,
    deployer: &D,
    registry: Arc<G>,
    manager: &V,
    capabilities: &Capabilities,
    directory: Arc<Directory>,
) where
    R: Runs,
    D: ContainerDeployer,
    D::Error: Into<Error>,
    G: ImageRegistry + 'static,
    G::Error: Into<Error>,
    V: VolumeManager,
    V::Error: Into<Error>,
{
    let scope = Arc::new(scope);

    if !capabilities.containers {
        send(&scope, R::unsupported()).await;
        scope.send_response_finish().await;
        return;
    }

    if let Err(error) = check(request) {
        send(&scope, R::error(&error)).await;
        scope.send_response_finish().await;
        return;
    }

    let mounts = request.volume_mounts.iter().map(|mount| (mount.volume_name.as_str(), mount.mode));
    let held = match Held::take(manager, client_identity, mounts, capabilities).await {
        Ok(held) => held,
        Err(Refused::Held(name)) => {
            send(&scope, R::volume_held(&VolumeHeld { name })).await;
            scope.send_response_finish().await;
            return;
        }
        Err(Refused::Mounted(name)) => {
            send(&scope, R::volume_mounted(&VolumeMounted { name })).await;
            scope.send_response_finish().await;
            return;
        }
        Err(Refused::FuseServed(name)) => {
            send(&scope, R::volume_fuse_served(&VolumeFuseServed { name })).await;
            scope.send_response_finish().await;
            return;
        }
        Err(Refused::MountUnsupported(name)) => {
            send(&scope, R::volume_mount_unsupported(&VolumeMountUnsupported { name })).await;
            scope.send_response_finish().await;
            return;
        }
        Err(Refused::Mode(refused)) => {
            send(&scope, R::volume_mode(&refused)).await;
            scope.send_response_finish().await;
            return;
        }
        Err(Refused::Error(error)) => {
            send(&scope, R::error(&error)).await;
            scope.send_response_finish().await;
            return;
        }
    };

    let identity: Arc<Identity> = Arc::new(client_identity.clone());
    let prepared = match setup::prepare::<R, D, G, V::Volume>(&scope, &identity, request, arguments, deployer, &registry, &held, &directory)
        .await
    {
        Ok(prepared) => prepared,
        Err(error) => {
            held.give_back().await;
            send(&scope, R::error(&error)).await;
            scope.send_response_finish().await;
            return;
        }
    };

    let id = prepared.id.clone();
    send(&scope, R::id(&Id { id: id.clone() })).await;

    // The container was registered with the caller in the setup; what
    // it declared is not needed again.
    let Begun {
        begin,
        asks,
        chunks,
        activity,
        dependencies: _,
        secrets: _,
        permissions: _,
    } = prepared.begun;
    let run = Arc::new(Run::new(
        Arc::clone(&scope),
        identity,
        Arc::clone(&directory),
        prepared.proxy,
        begin,
        prepared.watched,
        Ledger::new(request),
    ));
    // Every task of the run holds the run, so a run that never reaches
    // its teardown would be held by its own tasks for good: the
    // shutdown is a release too.
    let shutdown = Release::new({
        let run = Arc::clone(&run);
        async move { run.shutdown().await }
    });
    run.spawn(relay::relay::<R>(Arc::clone(&run), asks)).await;
    for mount in prepared.mounts {
        run.spawn(relay::fuse::<R>(Arc::clone(&run), mount)).await;
    }
    if let Some(chunks) = chunks {
        run.spawn(relay::chunks(Arc::clone(&run), chunks)).await;
    }
    if let Some(activity) = activity {
        run.spawn(relay::activity(Arc::clone(&run), activity)).await;
    }
    let _end = serve::serve::<R>(&run).await;

    prepared.removal.now().await;
    prepared.stop.now().await;
    held.give_back().await;
    shutdown.now().await;
    scope.send_response_finish().await;
}
