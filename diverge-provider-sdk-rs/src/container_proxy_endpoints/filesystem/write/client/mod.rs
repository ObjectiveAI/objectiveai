//! The client side of a write: what the server sends.
//!
//! [`request`] opens it — the destination, once. [`channel_response`]
//! answers the one channel the proxy opens: the content.
//!
//! There is no `channel_request`. The server asks nothing more of a
//! write it started; the content is an answer, not an ask.
//!
//! # And, behind the `server` feature, a way to use it
//!
//! [`execute`] performs the exchange rather than describing it: hand
//! it the [`Handle`](crate::client::handle::Handle) to the proxy, the
//! path and the content, and get back whether the file landed. Every
//! other module here is types only, and this one still is unless the
//! provider's server was asked for — the same bargain
//! [`server`](crate::server) makes.

pub mod channel_response;
pub mod request;

#[cfg(feature = "server")]
pub mod execute;
