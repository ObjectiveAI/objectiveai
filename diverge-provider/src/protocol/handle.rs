//! One connection, served whole.

use std::fmt;
use std::net::IpAddr;
use std::sync::Arc;

use futures_util::StreamExt as _;

use diverge_sdk::provider::endpoints::info::server::response::Capabilities;
use super::acceptors::Acceptors;
use super::capacity_reporter::CapacityReporter;
use super::container_deployer::ContainerDeployer;
use super::directory::Directory;
use super::changes::Changes;
use super::image_checker::ImageChecker;
use super::image_manager::ImageManager;
use super::oci_store::OciStore;
use super::image_registry::ImageRegistry;
use diverge_sdk::wire::server::received::Received;
use diverge_sdk::wire::server::session::Session;
use super::volume_manager::VolumeManager;
use diverge_sdk::shared::Identity;
use diverge_sdk::wire::decode::Decode;
use crate::protocol::endpoints;
use diverge_sdk::provider::endpoints::ClientRequest;
use diverge_sdk::shared::error::Error;

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
/// `client_identity` is the other end's [`Identity`], as the
/// connection's handshake verified it — made before the session, in
/// the host's accept or dial — and every handler receives it. `address`
/// is the peer the socket came from: it rides the announcement of a
/// daemon connection to its acceptor. It is a signal rather than an
/// identity.
///
/// The rest are what the provider supplies, shared because scopes run
/// concurrently and the traits — returning `impl Future` — cannot be
/// boxed behind one pointer — the [`CapacityReporter`] among them,
/// which `capacity` asks for the caller's room; the [`Capabilities`]
/// it holds out, which `info` answers and every handler that serves
/// an optional part of the protocol enforces — the image store, the registries, the FUSE
/// mounts and serves, the volume modes and what may hold a volume of
/// each; the [`Directory`], one per provider,
/// through which a FUSE serve or a transfer on this connection finds a run
/// on any other of the same caller's; the two [`Changes`], one for
/// volumes and one for images, through which a listing on this
/// connection hears of a change made on any other; and the
/// [`Acceptors`], one per provider likewise, through which a daemon
/// connecting on this connection finds one accepting on any other.
///
/// # The handshake is done before
///
/// The connection reaching here is past its
/// [`handshake`](diverge_sdk::wire::handshake): each end's token, the
/// connector's first, verified. A token arriving after it is
/// [`HandleError::InvalidAuthorize`]: serving stops, the teardown below
/// runs exactly as it would for the connection ending, and the error
/// says why it ran early.
pub async fn handle<D, V, I, M, O, R, C>(
    mut session: Session,
    client_identity: Arc<Identity>,
    address: IpAddr,
    deployer: Arc<D>,
    volume_manager: Arc<V>,
    image_checker: Arc<I>,
    image_manager: Arc<M>,
    oci_store: Arc<O>,
    image_registry: Arc<R>,
    capacity: Arc<C>,
    directory: Arc<Directory>,
    volume_changes: Arc<Changes>,
    image_changes: Arc<Changes>,
    acceptors: Arc<Acceptors>,
    capabilities: Arc<Capabilities>,
) -> Result<(), HandleError>
where
    D: ContainerDeployer + 'static,
    D::Error: Into<Error>,
    V: VolumeManager + 'static,
    V::Error: Into<Error>,
    I: ImageChecker + 'static,
    I::Error: Into<Error>,
    M: ImageManager + 'static,
    M::Error: Into<Error>,
    O: OciStore + 'static,
    R: ImageRegistry + 'static,
    R::Error: Into<Error>,
    C: CapacityReporter + 'static,
{
    // Owned here, so the drain at the bottom is over everything this
    // started and nothing else.
    let mut scopes = tokio::task::JoinSet::new();
    // How the loop ended: the connection going is [`Ok`], a token
    // arriving after the handshake is not — and both endings run the
    // same teardown below, which is why this is a variable rather than
    // two exits.
    let mut outcome = Ok(());

    while let Some(received) = session.next().await {
        // Finished ones, so the set does not grow for the life of the
        // connection.
        while scopes.try_join_next().is_some() {}

        let (payload, scope) = match received {
            Received::Request(payload, scope) => (payload, scope),
            // Each end sends ONE token, in the handshake that was done
            // before this loop. Another is a peer disagreeing with this
            // one about what the handshake is, and nothing after that
            // disagreement is worth serving.
            Received::Auth(_) => {
                outcome = Err(HandleError::InvalidAuthorize);
                break;
            }
        };

        // The decode borrows `payload` and nothing borrows `scope`, so
        // every arm is free to move the scope into its task with the
        // decoded request in hand.
        match ClientRequest::decode(&payload)
            .unwrap_or_else(|error| match error {})
        {
            ClientRequest::ContainersAgentsRun(frame) => {
                let identity = Arc::clone(&client_identity);
                let deployer = Arc::clone(&deployer);
                let registry = Arc::clone(&image_registry);
                let manager = Arc::clone(&volume_manager);
                let directory = Arc::clone(&directory);
                let capabilities = Arc::clone(&capabilities);
                scopes.spawn(async move {
                    endpoints::containers::agents::handle(
                        scope, frame, &identity, &*deployer, &*registry, &*manager, &capabilities, directory,
                    )
                    .await;
                });
            }
            ClientRequest::ContainersToolsRun(frame) => {
                let identity = Arc::clone(&client_identity);
                let deployer = Arc::clone(&deployer);
                let registry = Arc::clone(&image_registry);
                let manager = Arc::clone(&volume_manager);
                let directory = Arc::clone(&directory);
                let capabilities = Arc::clone(&capabilities);
                scopes.spawn(async move {
                    endpoints::containers::tools::handle(
                        scope, frame, &identity, &*deployer, &*registry, &*manager, &capabilities, directory,
                    )
                    .await;
                });
            }
            ClientRequest::ContainersFuseServe(frame) => {
                let identity = Arc::clone(&client_identity);
                let directory = Arc::clone(&directory);
                scopes.spawn(async move {
                    endpoints::containers::fuse_serve::handle(scope, frame, &identity, directory).await;
                });
            }
            ClientRequest::VolumesList(_) => {
                let identity = Arc::clone(&client_identity);
                let manager = Arc::clone(&volume_manager);
                let changes = Arc::clone(&volume_changes);
                let capabilities = Arc::clone(&capabilities);
                scopes.spawn(async move {
                    endpoints::volumes::list::handle(
                        scope, &identity, &*manager, changes, &capabilities,
                    )
                    .await;
                });
            }
            ClientRequest::VolumesStat(frame) => {
                let identity = Arc::clone(&client_identity);
                let manager = Arc::clone(&volume_manager);
                let capabilities = Arc::clone(&capabilities);
                scopes.spawn(async move {
                    endpoints::volumes::stat::handle(
                        scope, frame, &identity, &*manager, &capabilities,
                    )
                    .await;
                });
            }
            ClientRequest::VolumesRead(frame) => {
                let identity = Arc::clone(&client_identity);
                let manager = Arc::clone(&volume_manager);
                let capabilities = Arc::clone(&capabilities);
                scopes.spawn(async move {
                    endpoints::volumes::read::handle(
                        scope, frame, &identity, &*manager, &capabilities,
                    )
                    .await;
                });
            }
            ClientRequest::VolumesWrite(frame) => {
                let identity = Arc::clone(&client_identity);
                let manager = Arc::clone(&volume_manager);
                let capabilities = Arc::clone(&capabilities);
                scopes.spawn(async move {
                    endpoints::volumes::write::handle(
                        scope, frame, &identity, &*manager, &capabilities,
                    )
                    .await;
                });
            }
            ClientRequest::VolumesFiletree(frame) => {
                let identity = Arc::clone(&client_identity);
                let manager = Arc::clone(&volume_manager);
                let capabilities = Arc::clone(&capabilities);
                scopes.spawn(async move {
                    endpoints::volumes::filetree::handle(
                        scope, frame, &identity, &*manager, &capabilities,
                    )
                    .await;
                });
            }
            ClientRequest::VolumesFuseServe(frame) => {
                let identity = Arc::clone(&client_identity);
                let manager = Arc::clone(&volume_manager);
                let capabilities = Arc::clone(&capabilities);
                scopes.spawn(async move {
                    endpoints::volumes::fuse_serve::handle(
                        scope, frame, &identity, &*manager, &capabilities,
                    )
                    .await;
                });
            }
            ClientRequest::VolumesCreateCapacity(_) => {
                let identity = Arc::clone(&client_identity);
                let manager = Arc::clone(&volume_manager);
                let capabilities = Arc::clone(&capabilities);
                scopes.spawn(async move {
                    endpoints::volumes::create_capacity::handle(
                        scope, &identity, &*manager, &capabilities,
                    )
                    .await;
                });
            }
            ClientRequest::VolumesCreate(frame) => {
                let identity = Arc::clone(&client_identity);
                let manager = Arc::clone(&volume_manager);
                let changes = Arc::clone(&volume_changes);
                let capabilities = Arc::clone(&capabilities);
                scopes.spawn(async move {
                    endpoints::volumes::create::handle(
                        scope, frame, &identity, &*manager, changes, &capabilities,
                    )
                    .await;
                });
            }
            ClientRequest::VolumesEditCapacity(frame) => {
                let identity = Arc::clone(&client_identity);
                let manager = Arc::clone(&volume_manager);
                let capabilities = Arc::clone(&capabilities);
                scopes.spawn(async move {
                    endpoints::volumes::edit_capacity::handle(
                        scope, frame, &identity, &*manager, &capabilities,
                    )
                    .await;
                });
            }
            ClientRequest::VolumesEdit(frame) => {
                let identity = Arc::clone(&client_identity);
                let manager = Arc::clone(&volume_manager);
                let changes = Arc::clone(&volume_changes);
                let capabilities = Arc::clone(&capabilities);
                scopes.spawn(async move {
                    endpoints::volumes::edit::handle(
                        scope, frame, &identity, &*manager, changes, &capabilities,
                    )
                    .await;
                });
            }
            ClientRequest::VolumesDelete(frame) => {
                let identity = Arc::clone(&client_identity);
                let manager = Arc::clone(&volume_manager);
                let changes = Arc::clone(&volume_changes);
                let capabilities = Arc::clone(&capabilities);
                scopes.spawn(async move {
                    endpoints::volumes::delete::handle(
                        scope, frame, &identity, &*manager, changes, &capabilities,
                    )
                    .await;
                });
            }
            ClientRequest::ImagesCheck(frame) => {
                let identity = Arc::clone(&client_identity);
                let checker = Arc::clone(&image_checker);
                let capabilities = Arc::clone(&capabilities);
                scopes.spawn(async move {
                    endpoints::images::check::handle(
                        scope, frame, &identity, &*checker, &capabilities,
                    )
                    .await;
                });
            }
            ClientRequest::ImagesList(_) => {
                let identity = Arc::clone(&client_identity);
                let manager = Arc::clone(&image_manager);
                let changes = Arc::clone(&image_changes);
                let capabilities = Arc::clone(&capabilities);
                scopes.spawn(async move {
                    endpoints::images::list::handle(scope, &identity, &*manager, changes, &capabilities).await;
                });
            }
            ClientRequest::ImagesStat(frame) => {
                let identity = Arc::clone(&client_identity);
                let manager = Arc::clone(&image_manager);
                let capabilities = Arc::clone(&capabilities);
                scopes.spawn(async move {
                    endpoints::images::stat::handle(scope, frame, &identity, &*manager, &capabilities).await;
                });
            }
            ClientRequest::ImagesBuild(frame) => {
                let identity = Arc::clone(&client_identity);
                let manager = Arc::clone(&image_manager);
                let volumes = Arc::clone(&volume_manager);
                let changes = Arc::clone(&image_changes);
                let capabilities = Arc::clone(&capabilities);
                scopes.spawn(async move {
                    endpoints::images::build::handle(scope, frame, &identity, &*manager, &*volumes, changes, &capabilities).await;
                });
            }
            ClientRequest::ImagesDelete(frame) => {
                let identity = Arc::clone(&client_identity);
                let manager = Arc::clone(&image_manager);
                let changes = Arc::clone(&image_changes);
                let capabilities = Arc::clone(&capabilities);
                scopes.spawn(async move {
                    endpoints::images::delete::handle(scope, frame, &identity, &*manager, changes, &capabilities).await;
                });
            }
            ClientRequest::ImagesOci(_) => {
                let identity = Arc::clone(&client_identity);
                let store = Arc::clone(&oci_store);
                let capabilities = Arc::clone(&capabilities);
                scopes.spawn(async move {
                    endpoints::images::oci::handle(scope, &identity, store, &capabilities).await;
                });
            }
            ClientRequest::Info(_) => {
                let capabilities = Arc::clone(&capabilities);
                scopes.spawn(async move {
                    endpoints::info::handle(scope, &capabilities).await;
                });
            }
            ClientRequest::Capacity(_) => {
                let identity = Arc::clone(&client_identity);
                let reporter = Arc::clone(&capacity);
                scopes.spawn(async move {
                    endpoints::capacity::handle(scope, &identity, &*reporter).await;
                });
            }
            ClientRequest::DaemonsAccept(_) => {
                let identity = Arc::clone(&client_identity);
                let acceptors = Arc::clone(&acceptors);
                scopes.spawn(async move {
                    endpoints::daemons::accept::handle(scope, &identity, acceptors).await;
                });
            }
            ClientRequest::DaemonsConnect(frame) => {
                let identity = Arc::clone(&client_identity);
                let acceptors = Arc::clone(&acceptors);
                scopes.spawn(async move {
                    endpoints::daemons::connect::handle(scope, frame, &identity, address, acceptors).await;
                });
            }
            ClientRequest::Invalid(_) => scope.send_response_finish().await,
        }
    }

    // The connection is gone — or serving stopped early, over a
    // token that had no business arriving. Either way, dropping
    // the session is what tells every scope still being served: their
    // feeds close, and the handlers wind down on their own, containers
    // stopped and registries cleaned. The drain is never an abort, and
    // an early ending does not change that: the scopes in flight were
    // authorized, and the work they did was real.
    drop(session);
    while scopes.join_next().await.is_some() {}
    outcome
}

/// A connection that stopped being served early.
///
/// What [`handle`] returns instead of `()`. Once a connection is past
/// its handshake, nothing that happens on it is an error HERE but a
/// token arriving again, because every other failure belongs to some
/// scope and travels inside it. None of this reaches the peer: it is
/// for the provider, which drops the connection when this returns and
/// may want its log to say why.
#[derive(Debug)]
pub enum HandleError {
    /// A token arrived after the handshake.
    ///
    /// Each end sends one, before everything else. A peer that presents
    /// another is disagreeing about what the handshake is, and serving
    /// stopped where the disagreement started.
    InvalidAuthorize,
}

impl fmt::Display for HandleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HandleError::InvalidAuthorize => f.write_str("a token arrived after the handshake"),
        }
    }
}

impl std::error::Error for HandleError {}
