//! The client side of a container creation: what a client sends.
//!
//! [`request`] opens the scope. [`channel_response`] is what it sends
//! back on the channels the provider opens inside it — which, for a
//! caller-served image, is the whole of the image transfer.

pub mod channel_response;
pub mod request;
