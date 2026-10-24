//! What a client's channel response frame carries on the provider's
//! half of a connection.

/// One server frame the daemon sends on the connection, on its way
/// to the connector, of any type the wire has — its token, first, in
/// the handshake — as [`server::Owned`](crate::shared::daemons::server::Owned)
/// holds it: the frame's own bytes, with no tag in front, so that the
/// provider forwards them as they are.
pub type Frame<'a> = crate::wire::frame::server::ServerFrame<'a>;
