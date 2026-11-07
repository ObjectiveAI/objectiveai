//! The client side of a send: what a client sends.
//!
//! [`request`] opens the scope with the receiver and the message;
//! [`channel_request`] is the one channel a caller opens on it, the
//! cancel. [`execute`] performs the exchange: hand it a
//! [`Handle`](crate::wire::client::handle::Handle) and a
//! [`request::Frame`], and wait on, or cancel, the [`Sending`](execute::Sending)
//! it hands back.

pub mod channel_request;
pub mod request;

pub mod execute;
