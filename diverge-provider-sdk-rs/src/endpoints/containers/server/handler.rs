//! One run, from its request to its finish.

use std::sync::Arc;

use serde_json::Value;

use super::begin::Begun;
use super::check::check;
use super::family::Runs;
use super::held::{Held, Refused};
use super::run::{Run, send};
use super::{relay, serve, setup};
use crate::server::container::Container as _;
use crate::server::container_deployer::ContainerDeployer;
use crate::server::directory::Directory;
use crate::server::image_registry::ImageRegistry;
use crate::server::scope_handle::ScopeHandle;
use crate::server::volume_manager::VolumeManager;
use crate::shared::containers::request::Container;
use crate::shared::containers::response::{Id, VolumeHeld};
use crate::shared::error::Error;

/// Run the container the request describes for as long as the scope
/// lives, and end the scope.
///
/// In order:
///
/// 0. The request's mounts are checked — see [`check`] — or the run
///    is refused with the error, before anything is held.
///    Then every volume the request names is found and held, shared —
///    see [`Held`] — or the run is refused: with the name of a volume
///    held exclusively, or with an error for a name that is not the
///    caller's. Before anything is fetched or deployed. The holds are
///    the `Held`'s and are given back at every ending below.
/// 1. The container is brought up — see
///    [`setup::prepare`](super::setup::prepare): registry,
///    deploy, the proxy dialled, the family's begin, every mount. A
///    failure is the run's error, and the scope finishes on it.
/// 2. The id is minted, the container is entered in the directory —
///    with its proxy connection and its begin scope, for connectors —
///    and the id is sent. From here the container is running for the
///    caller.
/// 3. The relays are spawned: the proxy's asks on the begin scope,
///    each mount's asks, and — on an agent container — the agent's
///    chunks onto the main stream; on a tool container, the begin's
///    own end is listened for.
/// 4. The caller's channels are served until the run ends: a stop,
///    the container leaving, or the caller going away.
/// 5. The teardown, the same for every ending: the directory entry
///    removed, the container stopped, the repository released, the
///    volumes given back, every task ended, and the finish.
pub(crate) async fn run<R, D, G, V>(
    scope: ScopeHandle,
    client_identity: &str,
    request: &Container,
    agent: Option<Value>,
    deployer: &D,
    registry: &G,
    manager: &V,
    directory: Arc<Directory>,
) where
    R: Runs,
    D: ContainerDeployer,
    D::Error: Into<Error>,
    G: ImageRegistry,
    G::Error: Into<Error>,
    V: VolumeManager,
    V::Error: Into<Error>,
{
    let scope = Arc::new(scope);

    if let Err(error) = check(request) {
        send(&scope, R::error(&error)).await;
        scope.send_response_finish().await;
        return;
    }

    let names: Vec<&str> = request.volume_mounts.iter().map(|mount| mount.host_name.as_str()).collect();
    let held = match Held::take(manager, client_identity, names).await {
        Ok(held) => held,
        Err(Refused::Held(name)) => {
            send(&scope, R::volume_held(&VolumeHeld { name })).await;
            scope.send_response_finish().await;
            return;
        }
        Err(Refused::Error(error)) => {
            send(&scope, R::error(&error)).await;
            scope.send_response_finish().await;
            return;
        }
    };

    let prepared = match setup::prepare::<R, D, G, V::Volume>(&scope, client_identity, request, agent, deployer, registry, &held)
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

    let id = Directory::mint();
    let identity: Arc<str> = Arc::from(client_identity);
    directory.insert(
        id.clone(),
        Arc::clone(&scope),
        Arc::clone(&identity),
        prepared.proxy.clone(),
        prepared.begun.begin.tools(),
        prepared.ignore.clone(),
        Arc::clone(&prepared.watched),
    );
    send(&scope, R::id(&Id { id: id.clone() })).await;

    let Begun {
        begin,
        asks,
        chunks,
        finish,
    } = prepared.begun;
    let run = Arc::new(Run::new(
        Arc::clone(&scope),
        identity,
        Arc::clone(&directory),
        prepared.proxy,
        begin,
        prepared.ignore,
        prepared.watched,
    ));
    run.spawn(relay::relay::<R>(Arc::clone(&run), asks)).await;
    for mount in prepared.mounts {
        run.spawn(relay::fuse::<R>(Arc::clone(&run), mount)).await;
    }
    if let Some(chunks) = chunks {
        run.spawn(relay::chunks(Arc::clone(&run), chunks)).await;
    }
    if let Some(finish) = finish {
        // The begin scope ending, however it ends, is the container
        // gone.
        let over = Arc::clone(&run);
        run.spawn(async move {
            let _ = finish.await;
            over.over.notify_one();
        })
        .await;
    }
    let _end = serve::serve::<R>(&run).await;

    directory.remove(&id);
    prepared.container.stop().await;
    registry.release(&prepared.repository).await;
    held.give_back().await;
    run.shutdown().await;
    scope.send_response_finish().await;
}
