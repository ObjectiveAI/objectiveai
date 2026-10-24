//! One daemon connection, as the provider tells the acceptor of it.

use std::net::IpAddr;

use serde::{Deserialize, Serialize};

/// A connector arriving: what the provider says of it when it opens
/// the acceptor's half of the connection. The id correlates the two
/// channels the connection is, as a container program's daemon
/// connection is correlated; the identity and the address are the
/// provider's own word — the identity it authorized the connector's
/// connection under, the peer address it saw that connection come
/// from. Who the connector is to the acceptor is the relayed
/// connection's own handshake's to say.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Connection {
    /// The connection, by the id the provider minted: what the
    /// acceptor quotes when it opens its own half.
    pub connection_id: u32,
    /// The identity the provider authorized the connector under.
    pub identity: String,
    /// Where the connector's connection to the provider came from, as
    /// the provider saw it.
    pub address: IpAddr,
}
