//! What the daemon sends back on the channels a caller opens on its oci
//! scope, one module per kind: the provider scope's own answers, as the
//! source answered them. The cancel has no answer.

pub mod oci_blob;
pub mod oci_manifest;
pub mod oci_prove;
