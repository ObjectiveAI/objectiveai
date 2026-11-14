//! The client side of the daemon's read scope: what a client sends.
//!
//! [`request`] opens the scope, naming the one image it asks about and
//! the daemons not to ask; [`channel_request`] is what the client opens
//! on it: the three OCI asks, and the cancel that ends the scope.
//!
//! # And a way to use it
//!
//! [`execute`] performs the exchange rather than describing it: hand it
//! a [`Handle`](crate::wire::client::handle::Handle) and a
//! [`request::Frame`], get the answer — an
//! [`ExecuteHandle`](execute::ExecuteHandle) with one method per ask,
//! and the cancel, where the image is held.

pub mod channel_request;
pub mod request;

pub mod execute;
