//! What a server's channel response frame carries on the daemon's
//! half of a connection.

/// One client frame the connector sent on the connection, on its way
/// to the daemon, of any type the wire has — its token, first, in the
/// handshake — as [`client::Owned`](crate::shared::daemons::client::Owned)
/// holds it: the frame's own bytes, with no tag in front, as the
/// provider forwarded them.
pub type Frame<'a> = crate::wire::frame::client::ClientFrame<'a>;
