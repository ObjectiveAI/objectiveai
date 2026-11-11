//! What a provider sends back on the channels a caller opens on an
//! oci scope, one module per kind.
//!
//! [`oci_manifest`] and [`oci_blob`] hand over the manifests and
//! blobs the scope's image reaches; [`oci_prove`] the proof over it.
//! The cancel has no answer: nothing is sent on its channel.

pub mod oci_blob;
pub mod oci_manifest;
pub mod oci_prove;
