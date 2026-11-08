//! What a provider sends back on the channels a caller opens on a
//! pull, one module per kind.
//!
//! [`oci_has`] says whether the caller's Store holds the image, under
//! the fixed label `image`; [`oci_manifest`] and [`oci_blob`] hand
//! it over. The disconnect has no answer: nothing is sent on its
//! channel.

pub mod oci_blob;
pub mod oci_has;
pub mod oci_manifest;
