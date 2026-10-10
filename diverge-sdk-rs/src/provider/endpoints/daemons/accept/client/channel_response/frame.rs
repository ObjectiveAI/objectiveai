//! What a client's channel response frame carries on the provider's
//! half of a connection.

/// One server frame the daemon sends on the connection, on its way
/// to the connector. See
/// [`daemon::server::Frame`](crate::shared::containers::daemon::server::Frame):
/// the frame's own bytes, with no tag in front, so that the provider
/// forwards them as they are.
pub type Frame<'a> = crate::shared::containers::daemon::server::Frame<'a>;
