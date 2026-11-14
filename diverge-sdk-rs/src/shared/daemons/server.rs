//! One message of a relayed daemon connection, held.

use bytes::Bytes;

/// One message the acceptor sends, held whole and unread: its bytes as the
/// server half wrote them to its connection. The first of a connection
/// are the handshake's frames, in the clear; every one after is a
/// frame sealed under the connection's keys — see
/// [`handshake`](crate::wire::handshake) — which nothing between the
/// two daemons can read. A relay forwards them as they are, and this
/// type says nothing of what they hold.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Owned(Bytes);

impl Owned {
    /// The message `bytes` are, copied.
    pub fn of(bytes: &[u8]) -> Owned {
        Owned(Bytes::copy_from_slice(bytes))
    }

    /// The message's bytes, for a wire.
    pub fn bytes(&self) -> &Bytes {
        &self.0
    }
}
