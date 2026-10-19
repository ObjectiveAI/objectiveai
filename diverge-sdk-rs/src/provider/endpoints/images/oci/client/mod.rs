//! The client side of an image pull: what a client sends.
//!
//! [`request`] opens the scope, and it is empty; [`channel_request`]
//! is what the client opens on it: the three OCI asks, and the
//! disconnect that ends the scope. There is no `response` here the
//! way there is on the other side: the provider's answers come on
//! the asks' channels.
//!
//! # And a way to use it
//!
//! [`execute`] performs the exchange rather than describing it: hand
//! it a [`Handle`](crate::wire::client::handle::Handle) and a
//! [`request::Frame`], get an [`ExecuteHandle`](execute::ExecuteHandle)
//! with one method per ask, and the disconnect.
//!
//! Every other module in [`endpoints`](crate::provider::endpoints) is types only,
//! and this one still is unless a caller asked for the half of the
//! crate that can hold a socket — the same bargain
//! [`client`](crate::wire::client) itself makes.

pub mod channel_request;
pub mod request;

pub mod execute;
