//! What a caller sends back during a creation.
//!
//! One module per kind of channel the provider opens. [`oci`] serves
//! the image when it is the caller's to serve; [`authorize`] answers a
//! question; [`write_bytes`] streams the content of a file being written.

pub mod authorize;
pub mod oci;
pub mod write_bytes;
