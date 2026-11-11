//! The client side of an announce: what a client sends.
//!
//! [`request`] opens the scope, and carries the announcement;
//! [`channel_request`] is the one channel a caller opens on it, the
//! cancel: an announce has no end of its own, so the caller says when
//! it is done. [`execute`] performs the exchange: hand it a
//! [`Handle`](crate::wire::client::handle::Handle) and a
//! [`request::Frame`], keep the stream it hands back for the scope's
//! end, and cancel it with the cancel it hands back beside.

pub mod channel_request;
pub mod request;

pub mod execute;
