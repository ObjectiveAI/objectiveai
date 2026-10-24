//! The daemon as an acceptor: other daemons' connections through a
//! provider, judged and served.

use std::pin::Pin;
use std::sync::Arc;

use bytes::Bytes;
use diverge_sdk::provider::client::Acceptor;
use diverge_sdk::shared::daemons::{Connection, client, server};
use diverge_sdk::wire::connection::Connection as Wire;
use diverge_sdk::wire::handshake::{self, Handshook};
use futures_util::{Stream, stream};
use tokio::sync::mpsc::{self, UnboundedReceiver};

use crate::daemon::Daemon;
use crate::judge::{Admitted, Expected};
use crate::serve;

/// The daemon's half of one relayed connection: its server frames,
/// its handshake token first, for as long as the session lives.
pub type AcceptedFrames = Pin<Box<dyn Stream<Item = server::Owned> + Send>>;

/// What takes the connections a provider announces on the daemon's
/// accept scope. The rules:
///
/// 1. **The handshake.** The relayed connection is carried in-process
///    as a [`Connection::Local`](Wire::Local) and opened by the
///    [`handshake`](diverge_sdk::wire::handshake) as the connectee:
///    the connector's token, its first frame, is verified against the
///    address the provider saw, and the daemon's own token from the
///    same issuer is the first frame the daemon sends.
/// 2. **Admission.** The connector must be an account of this
///    daemon's — another daemon reaches this one through the accounts
///    system, and through nothing else. A provider's identity, and an
///    identity that is nothing here, are declined.
/// 3. **The session.** An admitted connector is served as the client
///    it is, in-process, by the same session a socket gets.
///
/// A connection declined is the half finished with nothing on it.
pub struct DaemonAcceptor {
    /// The daemon.
    pub daemon: Arc<Daemon>,
}

impl Acceptor for DaemonAcceptor {
    type Frames = AcceptedFrames;

    async fn accept(&self, connection: Connection, mut from_connector: UnboundedReceiver<client::Owned>) -> Option<Self::Frames> {
        let (to_session, incoming) = mpsc::unbounded_channel::<Bytes>();
        let (outgoing, from_session) = mpsc::unbounded_channel::<Bytes>();
        tokio::spawn(async move {
            while let Some(owned) = from_connector.recv().await {
                if to_session.send(owned.bytes().clone()).is_err() {
                    break;
                }
            }
        });
        let Handshook { connection: wire, peer } =
            handshake::accept(Wire::Local { incoming, outgoing }, &self.daemon.authenticator.expecting(Expected::Account), connection.address).await.ok()?;
        let Ok(Some(Admitted::Client(who))) = self.daemon.authenticator.admit(&peer).await else {
            return None;
        };
        tokio::spawn(serve::client(wire, who, Arc::clone(&self.daemon)));
        Some(Box::pin(stream::unfold(from_session, |mut from_session| async move {
            loop {
                let bytes = from_session.recv().await?;
                if let Ok(owned) = server::Owned::read(&bytes) {
                    return Some((owned, from_session));
                }
            }
        })))
    }
}
