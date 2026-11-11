//! The client side of an oci scope: what a client sends.
//!
//! [`request`] opens the scope, naming the one image it asks about
//! and, where it is held, serves; [`channel_request`] is what the
//! client opens on it: the three OCI asks, and the cancel that ends
//! the scope. There is no `response` here the way there is on the
//! other side: the client answers nothing.
//!
//! # And a way to use it
//!
//! [`execute`] performs the exchange rather than describing it: hand
//! it a [`Handle`](crate::wire::client::handle::Handle) and a
//! [`request::Frame`], get the answer — an
//! [`ExecuteHandle`](execute::ExecuteHandle) with one method per ask,
//! and the cancel, where the image is held.
//!
//! Every other module in [`endpoints`](crate::provider::endpoints) is types only,
//! and this one still is unless a caller asked for the half of the
//! crate that can hold a socket — the same bargain
//! [`client`](crate::wire::client) itself makes.

pub mod channel_request;
pub mod request;

pub mod execute;
