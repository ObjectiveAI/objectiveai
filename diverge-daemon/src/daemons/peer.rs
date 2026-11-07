//! One remote daemon connected to, as this daemon holds it.

use diverge_sdk::daemon::endpoints::daemons::DaemonId;
use diverge_sdk::provider::endpoints::daemons::connect::client::execute::Connected;
use diverge_sdk::shared::Identity;
use tokio::sync::watch;

/// A connection to another daemon, held while it lasts: the ID of the
/// record it was opened for, the provider it went through, and the
/// connection — the daemon-wire handle every request to that daemon
/// rides, and its end. The connection is our account the record names,
/// which the remote holds as one of its accounts.
pub struct Peer {
    /// The record's ID.
    pub daemon: DaemonId,
    /// The provider the connection went through.
    pub provider: Identity,
    /// The connection.
    pub connected: Connected,
    /// Turned `true` when the daemon forgets the peer — its record
    /// deleted, or a newer connection in its place: what its
    /// watcher waits on beside the connection's end, so the daemon's own
    /// hold on the connection goes with it, and the connection closes
    /// once nothing else uses it.
    pub closer: watch::Sender<bool>,
}

impl std::fmt::Debug for Peer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Peer")
            .field("daemon", &self.daemon)
            .field("provider", &self.provider)
            .finish_non_exhaustive()
    }
}
