//! Where the daemon stands with a provider: no connection, one it
//! uses, or one it holds open and never uses.

use serde::{Deserialize, Serialize};

use crate::provider::endpoints::info::server::response::Frame as Info;

/// The daemon's standing with one provider, as a list or a get
/// reports it: no connection; a connection the daemon uses, with
/// what the provider answered the daemon's `info` ask; or a
/// connection the daemon holds open and never uses, the provider
/// having answered a version that is not the daemon's own.
///
/// # The info is the whole of what the daemon knows
///
/// `info` is the provider's answer as it came — its version and its
/// capabilities, see
/// [`Frame`](crate::provider::endpoints::info::server::response::Frame)
/// — asked once per connection and cached for the connection's
/// life. A provider's capabilities may differ by the identity it is
/// asked as, and a connection is one identity, so what is cached is
/// what this daemon, as this identity, is held out. The daemon asks
/// a provider nothing its cached capabilities exclude, and a provider
/// counts as connected only once the answer is cached: a socket the
/// daemon is still asking on reads as `disconnected`.
///
/// # A different version is a socket kept and nothing more
///
/// A provider whose `version` is not the daemon's is used for
/// nothing — no container, no volume, no image, no daemon reached
/// through it — for as long as the versions differ; the daemon keeps
/// the socket so the provider is not dialled again and again, and
/// reports it so, with the answer that says why.
///
/// # On the wire
///
/// An object tagged by `type`: `{"type":"disconnected"}`,
/// `{"type":"connected","info":{…}}`,
/// `{"type":"different_version","info":{…}}`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Status {
    /// The daemon holds no connection to the provider.
    Disconnected,
    /// The daemon holds a connection it uses, and this is what the
    /// provider answered its `info` ask.
    Connected {
        /// The provider's version and capabilities, as answered.
        info: Info,
    },
    /// The daemon holds a connection it never uses: the provider's
    /// version is not the daemon's, and this is what it answered.
    DifferentVersion {
        /// The provider's version and capabilities, as answered.
        info: Info,
    },
}

impl Status {
    /// Whether the daemon uses the provider now: `Connected`, and
    /// nothing else — a different version is as good as no
    /// connection.
    pub fn connected(&self) -> bool {
        matches!(self, Status::Connected { .. })
    }
}
