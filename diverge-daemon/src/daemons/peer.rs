//! One remote daemon connected to, as this daemon holds it.

use diverge_sdk::daemon::endpoints::providers::daemons::Link;
use diverge_sdk::provider::endpoints::daemons::connect::client::execute::Connected;

/// A connection to another daemon, held while it lasts: the name the
/// configuration gives the remote, the link it was opened through,
/// and the connection — the daemon-wire handle every request to that
/// daemon rides, and its end. The connection is this daemon's
/// identity at the remote's issuer, which the remote holds as one of
/// its accounts.
pub struct Peer {
    /// The remote's name in the configuration.
    pub daemon: String,
    /// The link the connection went through: the provider, and the
    /// remote's identity there.
    pub link: Link,
    /// The connection.
    pub connected: Connected,
}

impl std::fmt::Debug for Peer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Peer")
            .field("daemon", &self.daemon)
            .field("link", &self.link)
            .finish_non_exhaustive()
    }
}
