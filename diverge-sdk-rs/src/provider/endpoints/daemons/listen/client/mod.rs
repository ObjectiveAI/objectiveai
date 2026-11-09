//! The client side of a listen: what a client sends.
//!
//! [`request`] opens the scope, and carries nothing beyond its tag;
//! [`channel_request`] is the one channel a caller opens on it, the
//! cancel: a listen has no end of its own, so the caller says when it
//! is done. [`execute`] performs the exchange: hand it a
//! [`Handle`](crate::wire::client::handle::Handle) and a
//! [`request::Frame`], read the messages off the stream it hands back,
//! and cancel it with the cancel it hands back beside.

pub mod channel_request;
pub mod request;

pub mod execute;
