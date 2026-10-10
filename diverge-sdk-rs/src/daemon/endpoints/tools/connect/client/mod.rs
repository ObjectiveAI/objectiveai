//! The client side of a connect: what a client sends.
//!
//! [`request`] opens the scope, and [`channel_request`] is every
//! channel a client may open on it: the disconnect, and the five MCP
//! exchanges. A client sends nothing else; there is no `response` here
//! the way there is on the other side.
//!
//! # And a way to use it
//!
//! [`execute`] performs the exchange rather than describing it: hand it
//! a [`Handle`](crate::wire::client::handle::Handle) and the request,
//! and get a handle on the connected tool.

pub mod channel_request;
pub mod request;

pub mod execute;
