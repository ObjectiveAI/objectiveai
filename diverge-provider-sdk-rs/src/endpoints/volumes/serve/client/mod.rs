//! The client side of a serve: what a client sends.
//!
//! [`request`] opens the scope — the volume, once. [`channel_request`]
//! is what the client opens on it: the nine asks, and the stop that
//! ends the scope.
//!
//! # And, behind the `client` feature, a way to use it
//!
//! [`execute`] performs the exchange rather than describing it: hand
//! it a [`Handle`](crate::client::handle::Handle) and a
//! [`request::Frame`], get an [`ExecuteHandle`](execute::ExecuteHandle)
//! that asks one ask at a time and stops when dropped into its stop.

pub mod channel_request;
pub mod request;

#[cfg(feature = "client")]
pub mod execute;
