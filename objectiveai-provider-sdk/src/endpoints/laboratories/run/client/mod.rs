//! The client side of a laboratory run: what a client sends.
//!
//! [`request`] opens the scope. [`channel_request`] reaches into the
//! container once it runs. [`channel_response`] answers the channels
//! the provider opens — which, for a caller-served image, is the whole
//! of the image transfer.

pub mod channel_request;
pub mod channel_response;
pub mod request;
