//! What a client's channel response frame carries on the frames
//! channel of a connect.

/// One client frame the connector sends on the connection, on its
/// way to the acceptor. See
/// [`daemon::client::Frame`](crate::shared::containers::daemon::client::Frame):
/// the frame's own bytes, with no tag in front, so that the provider
/// forwards them as they are. The connector finishing the channel is
/// the connector hanging up.
pub type Frame<'a> = crate::shared::containers::daemon::client::Frame<'a>;
