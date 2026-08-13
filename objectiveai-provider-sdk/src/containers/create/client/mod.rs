//! The client side of a container creation: what a client sends.
//!
//! [`request`] opens the scope. [`response`] is what it sends back on
//! the channels the provider opens inside it.

pub mod request;
pub mod response;
