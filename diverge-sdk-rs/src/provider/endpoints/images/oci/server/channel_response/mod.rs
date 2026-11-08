//! What a provider sends back on the channels a caller opens on a
//! pull, one module per kind.
//!
//! [`oci_manifest`] and [`oci_blob`] hand over the manifests and
//! blobs the scope's image reaches. The cancel has no answer: nothing
//! is sent on its channel.

pub mod oci_blob;
pub mod oci_manifest;
