//! A connection past its handshake.

use super::Peer;
use crate::wire::connection::Connection;

/// The connection, handed back once its handshake is done — sealed,
/// so every frame from here on crosses the transport encrypted — and
/// the other end it is with.
pub struct Handshook {
    /// The connection, ready for a router or a session.
    pub connection: Connection,
    /// The other end, verified, and proven to hold the key its token
    /// names.
    pub peer: Peer,
}
