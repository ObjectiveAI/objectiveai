//! What a client's channel response frame carries on the frames
//! channel of a connect.

/// One client frame the connector sends on the connection, on its
/// way to the acceptor, of any type the wire has — its token, first,
/// in the handshake: the frame's own bytes, with no tag in front, so
/// that the provider forwards them as they are. The connector
/// finishing the channel is the connector hanging up.
pub type Frame<'a> = crate::wire::frame::client::ClientFrame<'a>;
