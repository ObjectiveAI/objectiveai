//! The client side of a terminal: what a client sends.
//!
//! [`request`] opens the scope, and [`channel_request`] is every
//! channel a client may open on it: the close, stdin, a resize.
//! [`execute`] performs the exchange: the output, and the handle that
//! writes.

pub mod channel_request;
pub mod execute;
pub mod request;
