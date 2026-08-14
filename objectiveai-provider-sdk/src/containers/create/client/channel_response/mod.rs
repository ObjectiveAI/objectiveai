//! What a caller sends back during a creation.
//!
//! One module per kind of channel the provider opens. [`oci`] serves
//! the image when it is the caller's to serve; [`authorize`] answers a
//! question.

pub mod authorize;
pub mod oci;
