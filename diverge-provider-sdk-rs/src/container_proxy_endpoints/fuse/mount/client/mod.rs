//! The client side of a mount: what the server sends.
//!
//! [`request`] opens the scope — a path and a kind, once.
//! [`channel_response`] answers the asks the proxy opens on it.
//!
//! There is no `channel_request`. The server opens no channel on a
//! mount; it has everything it needs from the path.

pub mod channel_response;
pub mod request;
