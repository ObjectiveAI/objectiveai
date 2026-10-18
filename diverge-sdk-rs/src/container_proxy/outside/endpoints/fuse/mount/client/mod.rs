//! The client side of a mount: what the server sends.
//!
//! [`request`] opens the scope — a path and a kind, once.
//! [`channel_response`] answers the asks the proxy opens on it.
//!
//! There is no `channel_request`. The server opens no channel on a
//! mount; it has everything it needs from the path.
//!
//! # And a way to use it
//!
//! [`execute`] performs the exchange rather than describing it: hand
//! it the [`Handle`](crate::wire::client::handle::Handle) to the proxy, the
//! path and the kind, and get back the mount's handle and the asks it
//! makes. Every other module here is types only, and this one still
//! is unless the provider's server was asked for — the same bargain
//! [`server`](crate::wire::server) makes.

pub mod channel_response;
pub mod request;

pub mod execute;
