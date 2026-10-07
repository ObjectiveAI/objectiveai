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
use tokio::sync::mpsc;

use super::Error;
use crate::daemon::Daemon;

/// A provider's connection, attached: counted as connected under its
/// identity, with the router not yet driven to the end.
///
/// What [`attach`] hands back once the provider has answered its
/// version. [`serve`](Self::serve) is the rest of the connection's
/// life, and takes the handle out at its end. Dropping this instead
/// ends the connection, since the router and its half of the socket
/// are here and nowhere else — and leaves the handle registered,
/// which is why the one thing that drops one, a dial task being
/// ended, takes the handle out itself: see
/// [`Live::stop_dial`](crate::daemon::Live::stop_dial).
#[must_use = "a connection that is not served ends when this is dropped"]
pub struct Attached {
    /// The daemon the handle is registered in.
    daemon: Arc<Daemon>,
    /// The provider, as registered.
    identity: Identity,
    /// The router, reading the socket until it ends.
    reading: Pin<Box<dyn Future<Output = Result<(), InvalidAuthorize>> + Send>>,
}

/// Hold `connection`, already past its handshake, as the connection
/// to the provider `identity`.
///
/// The socket is split into the SDK's client router and handle; the
/// provider is asked its version, with the router driven meanwhile,
/// and one that does not answer is not a provider, [`Error::Version`],
/// and the socket is dropped; then the handle is registered under the
/// identity, and what comes back is the rest of the connection's
/// life, to be served.
pub async fn attach(connection: Connection, identity: Identity, daemon: &Arc<Daemon>) -> Result<Attached, Error> {
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
        answered.map_err(Error::Version)?;
    }
    daemon.live.connect_provider(identity.clone(), handle).await;
    Ok(Attached {
        daemon: Arc::clone(daemon),
        identity,
        reading,
    })
}

impl Attached {
    /// Drive the connection to its end, and take the handle out.
    pub async fn serve(self) {
        let Attached {
            daemon,
            identity,
            reading,
        } = self;
        let _ = reading.await;
        daemon.live.disconnect_provider(&identity).await;
    }
}
