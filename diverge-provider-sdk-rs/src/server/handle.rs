//! One connection, served whole.

use std::fmt;
use std::net::IpAddr;
use std::sync::Arc;

use futures_util::StreamExt as _;

use super::authorization::{self, Authorization};
use super::container_deployer::ContainerDeployer;
use super::directory::Directory;
use super::image_checker::ImageChecker;
use super::image_registry::ImageRegistry;
use super::received::Received;
use super::session::Session;
use super::unbrokered_authorizer::UnbrokeredAuthorizer;
use super::volume_mount_manager::VolumeMountManager;
use crate::decode::Decode;
use crate::endpoints;
use crate::endpoints::ClientRequest;
use crate::frame::auth;
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
/// [`Authorization`] is which side of this connection authenticates,
/// and with what — see the handshake below, which is where the
/// `client_identity` every handler receives now comes from. `address`
/// is the peer the socket came from: it is handed to the
/// [`UnbrokeredAuthorizer`] beside the credential, and it rides a
/// connector's
/// [`Authorize`](crate::shared::containers::authorize::request::Authorize).
/// It is a signal rather than an identity.
///
/// The rest are the provider's capabilities, shared because scopes run
/// concurrently and the traits — returning `impl Future` — cannot be
/// boxed behind one pointer; and the [`Directory`], one per provider,
/// through which a connect on this connection finds a run on any
/// other.
///
/// # The handshake comes first
///
/// On an [`Outgoing`](Authorization::Outgoing) connection this end
/// dialled, so this end authenticates: the credential goes out as the
/// connection's first frame, before anything is read — nothing may
/// precede it, by the auth frame's own rule — and the identity was
/// carried in beside it, being a fact of the dial.
///
/// On an [`Incoming`](Authorization::Incoming) connection the peer
/// dialled, so the peer speaks first, and the first thing it says must
/// be who it is. A first frame that is not the credential, a
/// credential that will not decode, and a credential the authorizer
/// refuses are all [`Err`] — and all of them earn the peer NOTHING on
/// the wire. A request from an unauthenticated peer is dropped with
/// its scope unfinished, deliberately breaking the finish-always rule
/// every handler obeys: a peer that has not authenticated cannot make
/// this end compose a reply, and a finish frame is a reply. The close
/// that follows the provider dropping everything is the whole answer,
/// which is the no-answer rule the auth frame has documented from the
/// start.
///
/// After the handshake a credential is never legitimate again, on
/// either variant — a connection has one, presented by the side that
/// dialled. One arriving anyway is [`HandleError::InvalidAuthorize`]:
/// serving stops, the teardown below runs exactly as it would for the
/// connection ending, and the error says why it ran early.
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
/// answer — twelve endpoints have twelve error vocabularies, and an
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
/// already paid for, which is the wrong way round: the work was real,
/// and a caller that leaves does not un-spend it.
pub async fn handle<D, V, I, U, R>(
    mut session: Session,
    authorization: Authorization<U>,
    address: IpAddr,
    deployer: Arc<D>,
    volume_mount_manager: Arc<V>,
    image_checker: Arc<I>,
    image_registry: Arc<R>,
    directory: Arc<Directory>,
) -> Result<(), HandleError<U::Error>>
where
    D: ContainerDeployer + 'static,
    D::Error: Into<Error>,
    V: VolumeMountManager + 'static,
    V::Error: Into<Error>,
    I: ImageChecker + 'static,
    I::Error: Into<Error>,
    U: UnbrokeredAuthorizer,
    R: ImageRegistry + 'static,
    R::Error: Into<Error>,
{
    // The handshake, before anything else. What comes out of it is the
    // identity everything downstream receives — shared rather than
    // cloned per scope, because every handler borrows it for the
    // length of a call and a scope lives as long as it lives.
    let client_identity: Arc<str> = match authorization {
        Authorization::Outgoing { auth, client_identity } => {
            // This end dialled, so this end authenticates, and the
            // credential is the connection's first frame — sent before
            // the first read, because nothing may precede it.
            match &auth {
                authorization::Auth::Unbrokered(credential) => {
                    session.send_auth(auth::Auth::Unbrokered(credential)).await;
                }
            }
            client_identity.into()
        }
        Authorization::Incoming { unbrokered } => {
            // The peer dialled, so the peer speaks first, and the
            // first thing it says must be who it is.
            let Some(received) = session.next().await else {
                // Gone before saying anything. Nothing was judged and
                // nothing was served, which is the ordinary ending
                // everywhere in this crate.
                return Ok(());
            };
            let Received::Auth(payload) = received else {
                // A request from a peer that never authenticated. The
                // scope inside it drops UNFINISHED: a peer that has
                // not authenticated cannot make this end compose a
                // reply, and a finish frame is a reply.
                return Err(HandleError::Unauthenticated);
            };
            let credential = match auth::Auth::decode(&payload) {
                Ok(auth::Auth::Unbrokered(credential)) => credential,
                Err(error) => return Err(HandleError::Auth(error)),
            };
            match unbrokered.authorize(credential, address).await {
                Ok(identity) => identity.into(),
                Err(error) => {
                    return Err(HandleError::Unauthorized(error));
                }
            }
        }
    };
    // Owned here, so the drain at the bottom is over everything this
    // started and nothing else.
    let mut scopes = tokio::task::JoinSet::new();
    // How the loop ended: the connection going is [`Ok`], a credential
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
            // A connection has ONE credential, presented by the side
            // that dialled, before everything. A second one — or a
            // first on a connection where this end dialled — is a peer
            // disagreeing with this one about what the handshake is,
            // and nothing after that disagreement is worth serving.
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
                let manager = Arc::clone(&volume_mount_manager);
                let directory = Arc::clone(&directory);
                scopes.spawn(async move {
                    endpoints::containers::agents::run::server::handle::handle(
                        scope, frame, &identity, &*deployer, &*registry, &*manager, directory,
                    )
                    .await;
                });
            }
            ClientRequest::ContainersToolsRun(frame) => {
                let identity = Arc::clone(&client_identity);
                let deployer = Arc::clone(&deployer);
                let registry = Arc::clone(&image_registry);
                let manager = Arc::clone(&volume_mount_manager);
                let directory = Arc::clone(&directory);
                scopes.spawn(async move {
                    endpoints::containers::tools::run::server::handle::handle(
                        scope, frame, &identity, &*deployer, &*registry, &*manager, directory,
                    )
                    .await;
                });
            }
            ClientRequest::ContainersToolsConnect(frame) => {
                let identity = Arc::clone(&client_identity);
                let directory = Arc::clone(&directory);
                scopes.spawn(async move {
                    endpoints::containers::tools::connect::server::handle::handle(
                        scope, frame, &identity, address, directory,
                    )
                    .await;
                });
            }
            ClientRequest::VolumesList(_) => {
                let identity = Arc::clone(&client_identity);
                let manager = Arc::clone(&volume_mount_manager);
                scopes.spawn(async move {
                    endpoints::volumes::list::server::handle::handle(
                        scope, &identity, &*manager,
                    )
                    .await;
                });
            }
            ClientRequest::VolumesStat(frame) => {
                let identity = Arc::clone(&client_identity);
                let manager = Arc::clone(&volume_mount_manager);
                scopes.spawn(async move {
                    endpoints::volumes::stat::server::handle::handle(
                        scope, frame, &identity, &*manager,
                    )
                    .await;
                });
            }
            ClientRequest::VolumesCreateCapacity(_) => {
                let identity = Arc::clone(&client_identity);
                let manager = Arc::clone(&volume_mount_manager);
                scopes.spawn(async move {
                    endpoints::volumes::create_capacity::server::handle::handle(
                        scope, &identity, &*manager,
                    )
                    .await;
                });
            }
            ClientRequest::VolumesCreate(frame) => {
                let identity = Arc::clone(&client_identity);
                let manager = Arc::clone(&volume_mount_manager);
                scopes.spawn(async move {
                    endpoints::volumes::create::server::handle::handle(
                        scope, frame, &identity, &*manager,
                    )
                    .await;
                });
            }
            ClientRequest::VolumesEditCapacity(frame) => {
                let identity = Arc::clone(&client_identity);
                let manager = Arc::clone(&volume_mount_manager);
                scopes.spawn(async move {
                    endpoints::volumes::edit_capacity::server::handle::handle(
                        scope, frame, &identity, &*manager,
                    )
                    .await;
                });
            }
            ClientRequest::VolumesEdit(frame) => {
                let identity = Arc::clone(&client_identity);
                let manager = Arc::clone(&volume_mount_manager);
                scopes.spawn(async move {
                    endpoints::volumes::edit::server::handle::handle(
                        scope, frame, &identity, &*manager,
                    )
                    .await;
                });
            }
            ClientRequest::VolumesDelete(frame) => {
                let identity = Arc::clone(&client_identity);
                let manager = Arc::clone(&volume_mount_manager);
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

    // The connection is gone — or serving stopped early, over a
    // credential that had no business arriving. Either way, dropping
    // the session is what tells every scope still being served: their
    // feeds close, and the handlers wind down on their own, containers
    // stopped and registries cleaned. The drain is never an abort, and
    // an early ending does not change that: the scopes in flight were
    // authorized, and the work they did was real.
    drop(session);
    while scopes.join_next().await.is_some() {}
    outcome
}

/// A connection that was never served, or that stopped being.
///
/// What [`handle`] returns instead of `()`, and every variant is about
/// the handshake — once a connection is authenticated, nothing that
/// happens on it is an error HERE, because every failure after that
/// point belongs to some scope and travels inside it.
///
/// # None of this reaches the peer
///
/// The auth frame's rule: an accepted credential is followed by the
/// connection simply working, a rejected one by a close, and a peer
/// that has not authenticated cannot make this end compose a reply —
/// no bytes to amplify, no answer to read a reason out of. These
/// variants are for the PROVIDER, which is the party that drops the
/// connection when this returns and may want its log to say why.
#[derive(Debug)]
pub enum HandleError<E> {
    /// The peer's first frame was not its credential.
    ///
    /// Whatever it was — most likely a request, whose scope was
    /// dropped unfinished, see [`handle`] — the peer skipped the
    /// handshake, and nothing it sends is worth reading.
    Unauthenticated,
    /// The credential would not decode.
    ///
    /// A mode this version does not know, or a credential that was not
    /// UTF-8. Not a refusal — nothing was ever judged — but the same
    /// close, because a credential this end cannot read authenticates
    /// nobody.
    Auth(auth::AuthError),
    /// The authorizer refused the credential, and this is its reason.
    ///
    /// The one variant that is a decision rather than a defect, in the
    /// provider's own vocabulary — see
    /// [`UnbrokeredAuthorizer::Error`].
    Unauthorized(E),
    /// A credential arrived after the handshake.
    ///
    /// A connection has one, presented by the side that dialled,
    /// before everything else. A peer that presents another — or
    /// presents one at all on a connection this end dialled — is
    /// disagreeing about what the handshake is, and serving stopped
    /// where the disagreement started.
    InvalidAuthorize,
}

impl<E> fmt::Display for HandleError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HandleError::Unauthenticated => {
                f.write_str("the peer's first frame was not its credential")
            }
            HandleError::Auth(error) => {
                write!(f, "the credential could not be read: {error}")
            }
            HandleError::Unauthorized(_) => {
                f.write_str("the authorizer refused the credential")
            }
            HandleError::InvalidAuthorize => {
                f.write_str("a credential arrived after the handshake")
            }
        }
    }
}

/// [`Unauthorized`](HandleError::Unauthorized) is not the source, even
/// though it wraps the reason: `E` is unbounded, like every provider
/// error on this half, and a bound here would be charged to every
/// implementation for a `source` chain nobody reads.
impl<E> std::error::Error for HandleError<E>
where
    E: fmt::Debug,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            HandleError::Auth(error) => Some(error),
            HandleError::Unauthenticated
            | HandleError::Unauthorized(_)
            | HandleError::InvalidAuthorize => None,
        }
    }
}
