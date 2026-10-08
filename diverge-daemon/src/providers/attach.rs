//! A provider's socket, held as the caller's.

use std::future::Future;
use std::pin::{Pin, pin};
use std::sync::Arc;

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::provider::endpoints::version;
use diverge_sdk::wire::client::handle::Handle;
use diverge_sdk::wire::client::router::{InvalidAuthorize, Router};
use diverge_sdk::wire::connection::Connection;
use futures_util::StreamExt as _;
use futures_util::future::{self, Either};
use tokio::sync::{Notify, mpsc};

use super::Error;
use crate::daemon::Daemon;

/// A provider's connection, attached: holding its slot under its
/// identity, with the router not yet driven to the end.
///
/// What [`attach`] hands back once the provider has answered its
/// version. [`serve`](Self::serve) is the rest of the connection's
/// life — until the socket ends or the daemon evicts it — and gives
/// the slot back at its end. Dropping this instead ends the
/// connection, since the router and its half of the socket are here
/// and nowhere else — and leaves the slot taken, which is why the one
/// thing that drops one, a dial task being ended, gives the slot back
/// itself: see [`Live::stop_dial`](crate::daemon::Live::stop_dial).
#[must_use = "a connection that is not served ends when this is dropped"]
pub struct Attached {
    /// The daemon the slot is held in.
    daemon: Arc<Daemon>,
    /// The provider, as held.
    identity: Identity,
    /// The daemon's word to end the connection.
    evict: Arc<Notify>,
    /// The router, reading the socket until it ends.
    reading: Pin<Box<dyn Future<Output = Result<(), InvalidAuthorize>> + Send>>,
}

/// Hold `connection`, already past its handshake, as the connection
/// to the provider `identity`, holding `credential` — the hash of the
/// key an incoming provider presented — with it.
///
/// The slot is taken first: one held under the identity, or one
/// holding the credential, is [`Error::Held`], and the socket is
/// dropped without a word. Then the socket is split into the SDK's
/// client router and handle; the provider is asked its version, with
/// the router driven meanwhile, and one that does not answer is not a
/// provider, [`Error::Version`], the slot given back and the socket
/// dropped; then the handle fills the slot, the provider's volumes
/// listing is opened and kept for the connection's life, and what
/// comes back is the rest of the connection's life, to be served.
pub async fn attach(connection: Connection, identity: Identity, credential: Option<String>, daemon: &Arc<Daemon>) -> Result<Attached, Error> {
    let Some(evict) = daemon.live.take_provider(identity.clone(), credential).await else {
        return Err(Error::Held);
    };
    let (sink, stream) = connection.split();
    let (registration_sender, registration_receiver) = mpsc::unbounded_channel();
    let (closed_sender, closed_receiver) = mpsc::unbounded_channel();
    let router = Router::new(stream, registration_receiver, closed_sender);
    let handle = Handle::new(sink, registration_sender, closed_receiver);
    let mut reading: Pin<Box<dyn Future<Output = Result<(), InvalidAuthorize>> + Send>> = Box::pin(router.run());
    {
        let asked = pin!(version::client::execute::execute(&handle));
        let answered = match future::select(asked, reading.as_mut()).await {
            Either::Left((answered, _)) => answered,
            // The socket ended before the version came back.
            Either::Right((_, asked)) => asked.await,
        };
        if let Err(error) = answered {
            daemon.live.disconnect_provider(&identity).await;
            return Err(Error::Version(error));
        }
    }
    daemon.live.connect_provider(&identity, handle.clone()).await;
    tokio::spawn(crate::volumes::watch(Arc::clone(daemon), identity.clone(), handle));
    Ok(Attached {
        daemon: Arc::clone(daemon),
        identity,
        evict,
        reading,
    })
}

impl Attached {
    /// Drive the connection to its end — the socket ending, or the
    /// daemon's eviction, on which the router is dropped and the
    /// handle with the slot, which closes the socket — and give the
    /// slot back.
    pub async fn serve(self) {
        let Attached {
            daemon,
            identity,
            evict,
            reading,
        } = self;
        tokio::select! {
            _ = reading => {}
            () = evict.notified() => {}
        }
        daemon.live.disconnect_provider(&identity).await;
    }
}
