//! The client side of an agent container run: what a client sends.
//!
//! [`request`] opens the scope. [`channel_request`] reaches into the
//! container once it runs. [`channel_response`] answers the channels
//! the provider opens.
//!
//! # And, behind the `client` feature, a way to use it
//!
//! [`execute`] performs the exchange rather than describing it: hand
//! it a [`Handle`](crate::client::handle::Handle) and a
//! [`request::Frame`] and the caller's
//! [`Answerers`](crate::client::Answerers), get back what runs. Every other module in
//! [`endpoints`](crate::endpoints) is types only, and this one still is
//! unless a caller asked for the half of the crate that can hold a
//! socket — the same bargain [`client`](crate::client) itself makes.

pub mod channel_request;
pub mod channel_response;
pub mod request;

#[cfg(feature = "client")]
pub mod execute;
