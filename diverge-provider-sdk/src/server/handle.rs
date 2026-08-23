//! One connection, served whole.

use std::net::IpAddr;
use std::sync::Arc;

use futures_util::StreamExt as _;

use super::container::Container;
use super::container_deployer::ContainerDeployer;
use super::image_checker::ImageChecker;
use super::laboratories::Laboratories;
use super::session::Session;
use super::volume_manager::VolumeManager;
use crate::decode::Decode;
use crate::endpoints;
use crate::endpoints::ClientRequest;
use crate::shared::error::Error;

/// Serve a connection: every scope its client opens, for as long as it
/// stays.
///
/// The provider's whole loop, one call per connection. The
/// [`Session`] yields each request beside the scope that answers it;
/// this reads the request to learn which endpoint it is, and spawns
/// that endpoint's handler with the decoded request and whatever it
/// needs from the arguments here. It returns when the connection has
/// ended AND everything it started has wound down.
///
/// # What the arguments are
///
/// Two are connection facts the crate cannot establish: the
/// [`Session`] discards [`Auth`](crate::frame::client::ClientFrame::Auth)
/// frames — its documented gap — so `client_identity` is whatever the
/// provider authenticated at the upgrade, and `address` is the peer the
/// socket came from, which rides a connector's
/// [`Authorize`](crate::endpoints::laboratories::run::server::channel_request::Authorize)
/// and is a signal rather than an identity.
///
/// The rest are the provider's capabilities, shared because scopes run
/// concurrently and the traits — returning `impl Future` — cannot be
/// boxed behind one pointer. [`laboratories`](Laboratories) must be
/// the SAME registry across every connection the provider serves: a
/// connector may arrive on a different connection than its runner, and
/// two registries would be two worlds that cannot see each other's
/// laboratories.
///
/// # Every scope runs in its own task
///
/// Not a preference — polling the [`Session`] is what runs the
/// connection, so work done inline starves every scope of the frames
/// that would feed it, including its own. The session's documentation
/// makes the rule absolute, and this is the function that obeys it.
///
/// # A request nobody can read is finished, with nothing
///
/// [`ClientRequest::decode`] cannot fail; what it cannot read it
/// returns as [`Invalid`](ClientRequest::Invalid), and the answer to
/// one is a finish with nothing in front. There is no other honest
/// answer — eleven endpoints have eleven error vocabularies, and an
/// invalid request names none of them — and a bare finish is already
/// what the wire means by a request that could not be served. Every
/// executor reads it as its own "unanswered".
///
/// # The ending drains, and never aborts
///
/// When the session yields [`None`] the connection is gone, and this
/// DROPS the session — which severs every open scope's feeds, so the
/// handlers still running find their receivers closed and wind down
/// through their own teardown: containers stopped, registries cleaned,
/// finishes sent into a socket that is no longer listening, harmlessly.
///
/// Then it waits for all of them. Aborting instead would tear through
/// every one of those teardowns — and it would end runs a caller had
/// already paid for, which the
/// [`agentic_loop`](crate::endpoints::agentic_loop::run) handler
/// documents as the wrong way round: the work was real, and a caller
/// that leaves does not un-spend it.
pub async fn handle<D, V, I>(
    mut session: Session,
    client_identity: String,
    address: IpAddr,
    deployer: Arc<D>,
    volume_manager: Arc<V>,
    image_checker: Arc<I>,
    laboratories: Arc<Laboratories<D::Container>>,
) where
    D: ContainerDeployer + 'static,
    D::Error: Into<Error>,
    <D::Container as Container>::Error: Into<Error>,
    V: VolumeManager + 'static,
    V::Error: Into<Error>,
    I: ImageChecker + 'static,
    I::Error: Into<Error>,
{
    // Shared rather than cloned per scope: every handler borrows it for
    // the length of a call, and a scope lives as long as it lives.
    let client_identity: Arc<str> = client_identity.into();
    // Owned here, so the drain at the bottom is over everything this
    // started and nothing else.
    let mut scopes = tokio::task::JoinSet::new();

    while let Some((payload, scope)) = session.next().await {
        // Finished ones, so the set does not grow for the life of the
        // connection.
        while scopes.try_join_next().is_some() {}

        // The decode borrows `payload` and nothing borrows `scope`, so
        // every arm is free to move the scope into its task with the
        // decoded request in hand.
        match ClientRequest::decode(&payload)
            .unwrap_or_else(|error| match error {})
        {
            ClientRequest::AgenticLoopRun(frame) => {
                // The tag off the front: what remains is the request's
                // own JSON, which that handler forwards raw so a field
                // this crate does not model survives the trip.
                let body = payload.slice(1..);
                let identity = Arc::clone(&client_identity);
                let deployer = Arc::clone(&deployer);
                scopes.spawn(async move {
                    endpoints::agentic_loop::run::server::handle::handle(
                        scope, frame, body, &identity, &*deployer,
                    )
                    .await;
                });
            }
            ClientRequest::McpPluginRun(frame) => {
                let identity = Arc::clone(&client_identity);
                let deployer = Arc::clone(&deployer);
                scopes.spawn(async move {
                    endpoints::mcp_plugin::run::server::handle::handle(
                        scope, frame, &identity, &*deployer,
                    )
                    .await;
                });
            }
            ClientRequest::LaboratoriesRun(frame) => {
                let identity = Arc::clone(&client_identity);
                let deployer = Arc::clone(&deployer);
                let laboratories = Arc::clone(&laboratories);
                scopes.spawn(async move {
                    endpoints::laboratories::run::server::handle::handle(
                        scope,
                        frame,
                        &identity,
                        &*deployer,
                        &laboratories,
                    )
                    .await;
                });
            }
            ClientRequest::LaboratoriesConnect(frame) => {
                let laboratories = Arc::clone(&laboratories);
                scopes.spawn(async move {
                    endpoints::laboratories::connect::server::handle::handle(
                        scope,
                        frame,
                        address,
                        &laboratories,
                    )
                    .await;
                });
            }
            ClientRequest::VolumesList(_) => {
                let identity = Arc::clone(&client_identity);
                let manager = Arc::clone(&volume_manager);
                scopes.spawn(async move {
                    endpoints::volumes::list::server::handle::handle(
                        scope, &identity, &*manager,
                    )
                    .await;
                });
            }
            ClientRequest::VolumesWatch(frame) => {
                let identity = Arc::clone(&client_identity);
                let manager = Arc::clone(&volume_manager);
                scopes.spawn(async move {
                    endpoints::volumes::watch::server::handle::handle(
                        scope, frame, &identity, &*manager,
                    )
                    .await;
                });
            }
            ClientRequest::VolumesCreate(frame) => {
                let identity = Arc::clone(&client_identity);
                let manager = Arc::clone(&volume_manager);
                scopes.spawn(async move {
                    endpoints::volumes::create::server::handle::handle(
                        scope, frame, &identity, &*manager,
                    )
                    .await;
                });
            }
            ClientRequest::VolumesEdit(frame) => {
                let identity = Arc::clone(&client_identity);
                let manager = Arc::clone(&volume_manager);
                scopes.spawn(async move {
                    endpoints::volumes::edit::server::handle::handle(
                        scope, frame, &identity, &*manager,
                    )
                    .await;
                });
            }
            ClientRequest::VolumesDelete(frame) => {
                let identity = Arc::clone(&client_identity);
                let manager = Arc::clone(&volume_manager);
                scopes.spawn(async move {
                    endpoints::volumes::delete::server::handle::handle(
                        scope, frame, &identity, &*manager,
                    )
                    .await;
                });
            }
            ClientRequest::ImagesCheck(frame) => {
                let identity = Arc::clone(&client_identity);
                let checker = Arc::clone(&image_checker);
                scopes.spawn(async move {
                    endpoints::images::check::server::handle::handle(
                        scope, frame, &identity, &*checker,
                    )
                    .await;
                });
            }
            ClientRequest::Version(_) => {
                scopes.spawn(
                    endpoints::version::server::handle::handle(scope),
                );
            }
            ClientRequest::Invalid(_) => scope.send_response_finish().await,
        }
    }

    // The connection is gone. Dropping the session is what tells every
    // scope still being served: their feeds close, and the handlers
    // wind down on their own.
    drop(session);
    while scopes.join_next().await.is_some() {}
}
