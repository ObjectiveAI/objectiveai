//! What each ask's channel decodes to: the run's three OCI answers,
//! read as a caller reads them.
//!
//! One marker per ask, never constructed, so that
//! [`unary`](crate::provider::endpoints::containers::client::unary)
//! and [`ChannelStream`](crate::provider::endpoints::containers::client::ChannelStream)
//! read them. None has a refusal: the OCI exchange has no error
//! vocabulary, and a provider that does not hold a digest finishes
//! the channel with nothing.

mod oci_blob;
mod oci_has;
mod oci_manifest;

pub use oci_blob::*;
pub use oci_has::*;
pub use oci_manifest::*;
