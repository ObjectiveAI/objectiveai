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
use crate::judge::{Admitted, Expected, Hold};
use crate::serve;

/// The daemon's half of one relayed connection: its server frames,
/// its handshake token first, for as long as the session lives.
pub type AcceptedFrames = Pin<Box<dyn Stream<Item = server::Owned> + Send>>;

/// What takes the connections a provider announces on the daemon's
/// accept scope: every one, at once, and served on a task of its own.
/// The rules:
///
/// 1. **Taken at once.** The connection is answered with the stream
///    the task feeds before a frame of the connector's is read, since
///    the provider relays nothing until this end's half is open, and
///    the half opens only once the connection is answered: a
///    handshake waited for here would wait on the half that waits on
///    it. So nothing is judged here, and nothing declined.
/// 2. **The handshake.** On the task, the relayed connection is
///    carried in-process as a [`Connection::Local`](Wire::Local) and
///    opened by the [`handshake`](diverge_sdk::wire::handshake) as
///    the connectee: the connector's token, its first frame, is
///    verified against the address the provider saw, and the daemon's
///    own token — its one identity's — is the first frame the daemon
///    sends.
/// 3. **Admission.** The connector must be an account of this
///    daemon's — another daemon reaches this one through the accounts
///    system, and through no list — that no other daemon connector
///    holds; the connection then holds the account until it ends — see
///    [`DaemonAuthenticator`](crate::judge::DaemonAuthenticator) —
///    as a [`Hold`], so that it is given back however the task ends.
///    A client socket connected as the same account neither counts
///    nor blocks.
/// 4. **The session.** An admitted connector is served, in-process, by
///    the same session a socket gets, as that account, and the
///    account is let go when the session ends.
/// 5. **A refusal is the stream ending.** A connector whose handshake
///    fails is one the stream ends on with nothing sent; one whose
///    account went between the handshake and its admission, one it
///    ends on after the daemon's token. Either way the connection it
///    was told is open hangs up, and no frame says why.
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
        let (daemon, address) = (Arc::clone(&self.daemon), connection.address);
        tokio::spawn(async move {
            // Rule 2. A handshake that fails drops the connection, and
            // the stream's sender with it: rule 5.
            let Ok(Handshook { connection: wire, peer, .. }) = handshake::accept(Wire::Local { incoming, outgoing }, &daemon.authenticator.expecting(Expected::Relayed), address).await else {
                return;
            };
            // Rule 3.
            let hold = Hold::new(&daemon.authenticator, Expected::Relayed, peer.clone());
            let Ok(Some(Admitted::Client(who))) = daemon.authenticator.account(&peer).await else {
                // The account went between the handshake and here.
                hold.release().await;
                return;
            };
            // Rule 4.
            serve::client(wire, who, Arc::clone(&daemon)).await;
            hold.release().await;
        });
        // Rule 1.
        Some(Box::pin(stream::unfold(from_session, |mut from_session| async move {
            // Unread: after the handshake each is a sealed frame.
            let bytes = from_session.recv().await?;
            Some((server::Owned::of(&bytes), from_session))
        })))
    }
}
