//! The client side of a tool container begin: what the server
//! sends.
//!
//! [`request`] opens the scope. [`channel_request`] reaches into the
//! container once it has begun. [`channel_response`] answers the
//! channels the proxy opens.
//!
//! # And a way to use it
//!
//! [`execute`] performs the exchange rather than describing it: hand
//! it the [`Handle`](crate::wire::client::handle::Handle) to the proxy, and
//! get back the scope's handle and the asks the proxy opens. Every
//! other module here is types only, and this one still is unless the
//! provider's server was asked for — the same bargain
//! [`server`](crate::wire::server) makes.

pub mod channel_request;
pub mod channel_response;
pub mod request;

pub mod execute;
