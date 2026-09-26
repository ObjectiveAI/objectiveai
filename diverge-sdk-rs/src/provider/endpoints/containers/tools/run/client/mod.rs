//! The client side of a tool container run: what a client sends.
//!
//! [`request`] opens the scope. [`channel_request`] reaches into the
//! container once it runs. [`channel_response`] answers the channels
//! the provider opens.
//!
//! # And a way to use it
//!
//! [`execute`] performs the exchange rather than describing it: hand
//! it a [`Handle`](crate::wire::client::handle::Handle) and a
//! [`request::Frame`] and the caller's
//! [`Answerers`](crate::provider::client::Answerers), get back what runs. Every other module in
//! [`endpoints`](crate::provider::endpoints) is types only, and this one still is
//! unless a caller asked for the half of the crate that can hold a
//! socket — the same bargain [`client`](crate::wire::client) itself makes.

pub mod channel_request;
pub mod channel_response;
pub mod request;

pub mod execute;
