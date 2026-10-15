//! A provider's socket, held as the caller's.

use std::future::Future;
use std::pin::{Pin, pin};
use std::sync::Arc;

use diverge_sdk::daemon::endpoints::agents::logs::server::response::Identity;
use diverge_sdk::provider::endpoints::info;
use diverge_sdk::wire::client::handle::Handle;
use diverge_sdk::wire::client::router::{InvalidAuthorize, Router};
use diverge_sdk::wire::connection::Connection;
use futures_util::StreamExt as _;
use futures_util::future::{self, Either};
use tokio::sync::{Notify, mpsc};

use super::Error;
use crate::daemon::Daemon;

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
/// The slot is reserved first: one held under the identity, or one
/// holding the credential, is [`Error::Held`], and the socket is
/// dropped without a word, so that no second socket of one provider
/// is ever admitted — and a reserved slot reads as no connection
/// until it is filled. Then the socket is split into the SDK's client
/// router and handle; the provider is asked its info, with the router
/// driven meanwhile, and one that does not answer is not a provider,
/// [`Error::Info`], the slot given back and the socket dropped. A
/// provider that answers the daemon's own `VERSION` fills the slot
/// as connected, with the handle and the answer cached for the
/// connection's life: the daemon's accept of other daemons is opened
/// on the connection when the configuration says so, and the
/// provider's volumes listing is opened and kept, when its
/// capabilities keep volumes at all. A provider that answers another
/// version fills the slot as a different version — the handle kept so
/// the socket stays open, and nothing opened on it, ever. Either way
/// what comes back is the rest of the connection's life, to be
/// served.
pub async fn attach(connection: Connection, identity: Identity, credential: Option<String>, daemon: &Arc<Daemon>) -> Result<Attached, Error> {
    let Some(evict) = daemon.live.reserve_provider(identity.clone(), credential).await else {
        return Err(Error::Held);
    };
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
            return Err(Error::Info(error));
        }
    };
    if answer.version == VERSION {
        let info = daemon.live.connect_provider(&identity, handle.clone(), answer).await;
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
