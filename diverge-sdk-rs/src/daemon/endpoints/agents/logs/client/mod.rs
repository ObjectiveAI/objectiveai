//! The client side of a logs read: what a client sends.
//!
//! [`request`] opens the scope, and [`channel_request`] is the one
//! channel a client may open on it, the cancel. A client sends
//! nothing else; there is no `response` here the way there is on the
//! other side.

pub mod channel_request;
pub mod request;
