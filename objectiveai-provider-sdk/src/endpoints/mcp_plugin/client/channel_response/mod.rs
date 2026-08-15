//! What a caller sends back on the channels a provider opens.
//!
//! [`oci`] serves the image when it is the caller's to serve, and it
//! is the only one — a provider needs one thing from a caller
//! mid-scope.

pub mod oci;
