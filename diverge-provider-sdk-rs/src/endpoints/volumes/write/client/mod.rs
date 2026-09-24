//! The client side of a volume write: what a client sends.
//!
//! [`request`] opens it — the volume and the destination, once.
//! [`channel_response`] answers the one channel the provider opens:
//! the content.
//!
//! There is no `channel_request`. A client asks nothing more of a
//! write it started; the content is an answer, not an ask.
//!
//! # And, behind the `client` feature, a way to use it
//!
//! [`execute`] performs the exchange rather than describing it: hand
//! it a [`Handle`](crate::client::handle::Handle), a
//! [`request::Frame`] and the content, and get back whether the file
//! landed. Every other module here is types only, and this one still
//! is unless a caller asked for the half of the crate that can hold a
//! socket — the same bargain [`client`](crate::client) makes.

pub mod channel_response;
pub mod request;

#[cfg(feature = "client")]
pub mod execute;
