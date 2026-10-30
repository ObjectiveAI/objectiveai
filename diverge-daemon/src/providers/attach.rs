//! A provider's socket, held as the caller's.

use std::future::Future;
use std::pin::{Pin, pin};
use std::sync::Arc;

use diverge_sdk::daemon::agent_log::Identity;
use diverge_sdk::provider::endpoints::info;
use diverge_sdk::wire::client::handle::Handle;
use diverge_sdk::wire::client::router::{InvalidAuthorize, Router};
use diverge_sdk::wire::connection::Connection;
use futures_util::StreamExt as _;
use futures_util::future::{self, Either};
use tokio::sync::mpsc;

use super::Error;
use crate::daemon::Daemon;
use crate::judge::Expected;

/// The version a provider must answer to be used: this crate's own,
/// read from the manifest at compile time. The daemon and the
/// provider are versioned with the SDK, which is the specification,
/// so an equal string is a provider speaking the daemon's protocol
/// and any other is one the daemon will not speak to.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// A provider's connection, attached: holding its slot under its
/// identity, with the router not yet driven to the end.
///
/// What [`attach`] hands back once the provider has answered its
/// info. [`serve`](Self::serve) is the rest of the connection's
/// life — until the socket ends — and gives the slot back at its end.
/// Dropping this instead ends the connection, since the router and
/// its half of the socket are here and nowhere else — and leaves the
/// slot taken, which only the daemon's stop, ending every dial task,
/// ever does.
#[must_use = "a connection that is not served ends when this is dropped"]
pub struct Attached {
    /// The daemon the slot is held in.
    daemon: Arc<Daemon>,
    /// The provider, as held.
    identity: Identity,
    /// The router, reading the socket until it ends.
    reading: Pin<Box<dyn Future<Output = Result<(), InvalidAuthorize>> + Send>>,
}

/// Hold `connection`, already past its handshake, as the connection
/// to the provider `identity`.
///
/// The handshake holds the identity already, so no second connection
/// under it reaches here. The slot is reserved first, and a reserved
/// slot reads as no connection until it is filled. Then the socket is split into the SDK's client
/// router and handle; the provider is asked its info, with the router
/// driven meanwhile, and one that does not answer is not a provider,
/// [`Error::Info`], the slot given back and the socket dropped. A
/// provider that answers the daemon's own `VERSION` fills the slot
/// as connected, with the handle and the answer cached for the
/// connection's life: the provider's capacity is asked, and asked
/// again for the connection's life; the daemon's accept of other
/// daemons is opened on the connection when the configuration says
/// so; and the provider's volumes listing is opened and kept, when
/// its capabilities keep volumes at all. A provider that answers another
/// version fills the slot as a different version — the handle kept so
/// the socket stays open, and nothing opened on it, ever. Either way
/// what comes back is the rest of the connection's life, to be
/// served.
pub async fn attach(connection: Connection, identity: Identity, daemon: &Arc<Daemon>) -> Result<Attached, Error> {
    daemon.live.reserve_provider(identity.clone()).await;
    let (sink, stream) = connection.split();
    let (registration_sender, registration_receiver) = mpsc::unbounded_channel();
    let (closed_sender, closed_receiver) = mpsc::unbounded_channel();
    let router = Router::new(stream, registration_receiver, closed_sender);
    let handle = Handle::new(sink, registration_sender, closed_receiver);
    let mut reading: Pin<Box<dyn Future<Output = Result<(), InvalidAuthorize>> + Send>> = Box::pin(router.run());
    let answered = {
        let asked = pin!(info::client::execute::execute(&handle));
        match future::select(asked, reading.as_mut()).await {
            Either::Left((answered, _)) => answered,
            // The socket ended before the info came back.
            Either::Right((_, asked)) => asked.await,
        }
    };
    let answer = match answered {
        Ok(answer) => answer,
        Err(error) => {
            daemon.live.disconnect_provider(&identity).await;
            daemon.authenticator.release(Expected::Provider, &identity).await;
            return Err(Error::Info(error));
        }
    };
    if answer.version == VERSION {
        let (info, capacity) = daemon.live.connect_provider(&identity, handle.clone(), answer).await;
        tokio::spawn(super::poll_capacity(Arc::clone(daemon), handle.clone(), capacity));
        if daemon.accept_daemons {
            tokio::spawn(super::accept(Arc::clone(daemon), handle.clone()));
        }
        if info.capabilities.volumes() {
            tokio::spawn(crate::volumes::watch(Arc::clone(daemon), identity.clone(), handle));
        }
    } else {
        daemon.live.different_version_provider(&identity, handle, answer).await;
    }
    Ok(Attached {
        daemon: Arc::clone(daemon),
        identity,
        reading,
    })
}

impl Attached {
    /// Drive the connection to its end — the socket ending — and give
    /// the slot back, and the identity the handshake holds.
    pub async fn serve(self) {
        let Attached { daemon, identity, reading } = self;
        let _ = reading.await;
        daemon.live.disconnect_provider(&identity).await;
        daemon.authenticator.release(Expected::Provider, &identity).await;
    }
}
