//! The client side of a connection: what a connector sends.
//!
//! [`request`] opens the scope and asks to be let in.
//! [`channel_request`] reaches into the container once it is.
//! [`channel_response`] answers the one channel a provider opens
//! back — the content of a write.

pub mod channel_request;
pub mod channel_response;
pub mod request;
