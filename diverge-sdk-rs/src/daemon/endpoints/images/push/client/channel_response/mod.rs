//! What a client sends back during a push, one module per kind of
//! channel the daemon opens: the provider push's own answers, byte for
//! byte, since the daemon carries each to the provider as the client
//! wrote it. [`oci_manifest`] and [`oci_blob`] hand the image over,
//! and [`oci_prove`] proves the client holds it.

pub use crate::provider::endpoints::images::push::client::channel_response::{oci_blob, oci_manifest, oci_prove};
