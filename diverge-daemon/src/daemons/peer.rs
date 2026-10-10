//! One daemon connected to, as this daemon holds it.

use diverge_sdk::daemon::endpoints::providers::daemons::Link;
use diverge_sdk::provider::endpoints::daemons::connect::client::execute::Connected;

/// A connection to another daemon, held while it lasts: the record it
/// is of, the link it was opened through, and the connection — the
/// daemon-wire handle every request to that daemon rides, and its
/// end.
pub struct Peer {
    /// The record's name.
    pub name: String,
    /// The link the connection went through: the provider, and the
    /// remote's identity there.
    pub link: Link,
    /// The connection.
    pub connected: Connected,
}

impl std::fmt::Debug for Peer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Peer").field("name", &self.name).field("link", &self.link).finish_non_exhaustive()
    }
}
