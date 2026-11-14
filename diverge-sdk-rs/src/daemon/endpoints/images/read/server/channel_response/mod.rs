//! What the daemon sends back on the channels a caller opens on its oci
//! scope, one module per kind: the provider scope's own answers, as the
//! source answered them — save the document, which the daemon answers
//! from its own record where it has one. The cancel has no answer.

pub mod info;
pub mod oci_blob;
pub mod oci_manifest;
pub mod oci_prove;
