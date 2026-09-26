//! What the handshake hands back.

use crate::wire::connection::Connection;

/// A connection past its handshake, and who is on the other end.
///
/// What [`authorize`](super::authorize::authorize) returns: the
/// [`Connection`], ready to be split into a
/// [`Router`](super::router::Router) and a
/// [`Handle`](super::handle::Handle), and the provider's identity —
/// supplied by the caller when it dialled, or answered by its
/// [`UnbrokeredAuthorizer`](super::unbrokered_authorizer::UnbrokeredAuthorizer)
/// when the provider did. Two fields rather than a tuple, because a
/// caller keeps the identity for the life of the connection and
/// longer, and a name is what lets it say which is which.
#[derive(Debug)]
pub struct Authorized {
    /// The connection, authenticated and not yet split.
    pub connection: Connection,
    /// The provider's identity, as the caller knows it from here on.
    pub provider_identity: String,
}
