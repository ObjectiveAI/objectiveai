//! The client side of an agent container run: what a client sends.
//!
//! [`request`] opens the scope. [`channel_request`] reaches into the
//! container once it runs. [`channel_response`] answers the channels
//! the provider opens.

pub mod channel_request;
pub mod channel_response;
pub mod request;
