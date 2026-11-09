//! A connection past its handshake.

use super::Peer;
use crate::shared::listable::Listable;
use crate::wire::connection::Connection;

/// The connection, handed back once its handshake is done, and the
/// other end it is with.
pub struct Handshook {
    /// The connection, ready for a router or a session.
    pub connection: Connection,
    /// The other end, verified.
    pub peer: Peer,
    /// The other end's listable flag: whether it may be listed by a
    /// provider's [`daemons::list`](crate::provider::endpoints::daemons::list),
    /// and the description it is listed with. Only a provider honors
    /// it.
    pub listable: Listable,
}
