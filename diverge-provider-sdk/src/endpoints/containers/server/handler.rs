//! A run, from the request to the finish.

use std::sync::Arc;

use serde_json::Value;

use super::family::Runs;
use super::relay;
use super::run::{Run, send};
use super::serve;
use super::setup;
use crate::container_proxy::agent;
use crate::server::container::Container as _;
use crate::server::container_deployer::ContainerDeployer;
use crate::server::content_store::ContentStore;
use crate::server::directory::Directory;
use crate::server::image_registry::ImageRegistry;
use crate::server::scope_handle::ScopeHandle;
use crate::shared::containers::request::Container;
use crate::shared::containers::response::Id;
use crate::shared::error::Error;

/// Serve one run scope, whole, for either family.
///
/// In order, and the order is the point:
///
/// 1. [`setup::prepare`]: content, registry, deploy, the proxy
///    dialled, every FUSE mount made and serving. A failure is the run's `Error`,
///    then the finish, and nothing the caller may have opened
///    meanwhile is read — it is dropped with the scope, unanswered,
///    which is what a caller reads as the run never having been.
/// 2. For an agent container, the agent registered — `agent` is
///    `Some` — with the proxy's `/agent/register`; a refusal is the
///    container's own `Error`, and the container is stopped. Never
///    before every mount is complete: a registration, like every
///    channel the caller opens and every filetree above all, waits on
///    the last mount.
/// 3. The id minted, the container entered in the directory, the id
///    sent: from here the run is real to the caller and to any
///    connector.
/// 4. The container's asks relayed, and everything the caller opened
///    — from the start, held in the inbox until now — served, in the
///    order it was opened, until the caller stops, the container
///    leaves, or the caller goes away.
/// 5. Teardown, the same for every ending: the directory entry
///    removed, which ends every connector; the container stopped; the
///    repository released; the tasks ended; and the finish — bare,
///    because an ending after the id is never an error.
pub(crate) async fn run<R, D, S, G>(
    scope: ScopeHandle,
    client_identity: &str,
    request: &Container,
    agent: Option<Value>,
    deployer: &D,
    store: &S,
    registry: &G,
    directory: &Directory,
) where
    R: Runs,
    D: ContainerDeployer,
    D::Error: Into<Error>,
    S: ContentStore,
    S::Error: Into<Error>,
    G: ImageRegistry,
    G::Error: Into<Error>,
{
    let scope = Arc::new(scope);
    let prepared = match setup::prepare::<R, D, S, G>(&scope, client_identity, request, deployer, store, registry).await {
        Ok(prepared) => prepared,
        Err(error) => {
            send(&scope, R::error(&error)).await;
            scope.send_response_finish().await;
            return;
        }
    };

    if let Some(agent) = agent {
        let request = agent::register::request::Request { agent };
        if let Err(error) = agent::register::execute::execute(&prepared.client, &request).await {
            let error = match error {
                agent::register::execute::ExecuteError::Refused(error) => error,
                other => super::render::proxy(other),
            };
            prepared.container.stop().await;
            if let Some(repository) = &prepared.repository {
                registry.release(repository).await;
            }
            send(&scope, R::error(&error)).await;
            scope.send_response_finish().await;
            return;
        }
    }

    let id = Directory::mint();
    directory.insert(
        id.clone(),
        Arc::clone(&scope),
        prepared.container.address().to_string(),
        prepared.ignore.clone(),
    );
    send(&scope, R::id(&Id { id: id.clone() })).await;

    let run = Arc::new(Run::new(Arc::clone(&scope), prepared.client, prepared.ignore));
    run.spawn(relay::relay::<R>(Arc::clone(&run), prepared.asks)).await;
    let _end = serve::serve::<R>(&run).await;

    directory.remove(&id);
    prepared.container.stop().await;
    if let Some(repository) = &prepared.repository {
        registry.release(repository).await;
    }
    run.shutdown().await;
    scope.send_response_finish().await;
}
