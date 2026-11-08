//! What each ask's channel decodes to: the run's manifest and blob
//! answers, read as a caller reads them.
//!
//! One marker per ask, never constructed, so that
//! [`unary`](crate::provider::endpoints::containers::client::unary)
//! and [`ChannelStream`](crate::provider::endpoints::containers::client::ChannelStream)
//! read them. None has a refusal: the OCI exchange has no error
//! vocabulary, and a provider asked for a digest the image does not reach finishes
//! the channel with nothing.

mod oci_blob;
mod oci_manifest;

pub use oci_blob::*;
pub use oci_manifest::*;
