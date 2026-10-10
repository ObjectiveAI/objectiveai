//! What a server's channel response frame carries on the daemon's
//! half of a connection.

/// One client frame the connector sent on the connection, on its way
/// to the daemon. See
/// [`daemon::client::Frame`](crate::shared::containers::daemon::client::Frame):
/// the frame's own bytes, with no tag in front, as the provider
/// forwarded them.
pub type Frame<'a> = crate::shared::containers::daemon::client::Frame<'a>;
