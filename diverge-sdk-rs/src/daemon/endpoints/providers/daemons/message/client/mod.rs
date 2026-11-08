//! The client side of a message: what a client sends.
//!
//! [`request`] opens the scope, and [`channel_request`] is the one
//! channel a client opens on it, the cancel. [`execute`] performs the
//! exchange: hand it a [`Handle`](crate::wire::client::handle::Handle)
//! and the request, and wait on, or cancel, the
//! [`Pending`](crate::daemon::client::Pending) it hands back.

pub mod channel_request;
pub mod request;

pub mod execute;
