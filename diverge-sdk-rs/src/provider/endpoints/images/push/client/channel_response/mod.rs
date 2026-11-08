//! What a caller sends back during a push, one module per kind of
//! channel the provider opens.
//!
//! [`oci_manifest`] and [`oci_blob`] hand the image over, and
//! [`oci_prove`] proves the caller holds it. Each is the run's own
//! answer, from [`oci`](crate::shared::containers::oci): a push asks
//! what a run asks, less whether the caller holds the image.

pub mod oci_blob;
pub mod oci_manifest;
pub mod oci_prove;
