//! The daemon as an acceptor: other daemons' connections through a
//! provider, judged and served.

use std::sync::Arc;

use diverge_sdk::provider::client::Acceptor;
use diverge_sdk::shared::containers::daemon as pair;
use diverge_sdk::shared::daemons::{Connection, Mode};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::daemon::Daemon;
use crate::judge::{self, Peer};
use crate::serve::{self, PairFrames};

/// What takes the connections a provider announces on the daemon's
/// accept scope: each judged by the credential its mode carries and
/// the address the provider saw, as any client's is at the handshake,
/// and served as that client, in-process, by the same session a
/// socket gets. A credential that admits nothing, or admits a
/// provider rather than a client, is declined: the half finished with
/// nothing on it.
pub struct DaemonAcceptor {
    /// The daemon.
    pub daemon: Arc<Daemon>,
}

impl Acceptor for DaemonAcceptor {
    type Frames = PairFrames;

    async fn accept(&self, connection: Connection, from_connector: UnboundedReceiver<pair::client::Owned>) -> Option<Self::Frames> {
        let Mode::Unbrokered { credential } = connection.mode;
        let Ok(Some(Peer::Client(who))) = judge::admit_peer(&self.daemon.store, &credential, connection.address).await else {
            return None;
        };
        Some(serve::serve_pair(Arc::clone(&self.daemon), who, from_connector, None))
    }
}
