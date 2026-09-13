//! The client side of an agent container begin: what the server
//! sends.
//!
//! [`request`] opens the scope. [`channel_request`] reaches into the
//! container once it has begun. [`channel_response`] answers the
//! channels the proxy opens.

pub mod channel_request;
pub mod channel_response;
pub mod request;
