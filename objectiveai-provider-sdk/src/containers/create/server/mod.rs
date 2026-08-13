//! The server side of a container creation: what a provider sends.
//!
//! [`request`] is what it opens channels to ask the caller for while
//! assembling the image. [`response`] is the answer to the creation
//! itself.
//!
//! Under construction — [`response`] is still empty.

pub mod request;
pub mod response;
