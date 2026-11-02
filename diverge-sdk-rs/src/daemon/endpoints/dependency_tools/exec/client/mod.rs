//! The client side of an exec: what a client sends.
//!
//! [`request`] opens the scope, and [`channel_request`] is the one
//! channel a client may open on it, the cancel, which kills the command.
//! [`execute`] performs the exchange: the stream and its cancel.

pub mod channel_request;
pub mod execute;
pub mod request;
