//! One remote daemon connected to, as this daemon holds it.

use diverge_sdk::provider::endpoints::daemons::connect::client::execute::Connected;
use diverge_sdk::shared::Identity;

/// A connection to another daemon, held while it lasts: the name of the
/// record it was opened for, the provider it went through, and the
/// connection — the daemon-wire handle every request to that daemon
/// rides, and its end. The connection is our account the record names,
/// which the remote holds as one of its accounts.
pub struct Peer {
    /// The record's name.
    pub daemon: String,
    /// The provider the connection went through.
    pub provider: Identity,
    /// The connection.
    pub connected: Connected,
}

impl std::fmt::Debug for Peer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Peer")
            .field("daemon", &self.daemon)
            .field("provider", &self.provider)
            .finish_non_exhaustive()
    }
}
