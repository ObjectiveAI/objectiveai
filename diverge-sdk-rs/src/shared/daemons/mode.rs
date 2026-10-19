//! How a connecting daemon authenticates to the daemon it connects
//! to.

use serde::{Deserialize, Serialize};

/// The connector's way in: what it would present as the first frame
/// of a daemon connection, carried here instead, since a connection
/// relayed through a provider has no `Auth` frame of its own. One
/// object whose one member is the mode, by name:
/// `{"unbrokered":{"credential":…}}`. One mode today; a brokered mode
/// is a second member when the wire defines it, and nothing moves.
/// The provider relays it to the acceptor unread, and the acceptor
/// judges it as it judges a credential presented on a socket.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// The two daemons already know each other: the connector
    /// presents a credential of an account it holds on the acceptor.
    Unbrokered {
        /// The credential, as the connector would present it in an
        /// `Auth::Unbrokered`: the key the acceptor minted for the
        /// account, byte for byte.
        credential: String,
    },
}
