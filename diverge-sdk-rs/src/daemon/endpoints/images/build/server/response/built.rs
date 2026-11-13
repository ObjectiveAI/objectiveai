//! The image, built.

use serde::{Deserialize, Serialize};

/// The image built into the daemon's Store on the provider: the
/// provider's
/// [`Built`](crate::provider::endpoints::images::build::server::response::Built),
/// as it gave it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Built {
    /// The manifest digest of the image as the provider holds it,
    /// `<algorithm>:<hex>`: what a template's image names.
    pub digest: String,
    /// Its size in BYTES, as the provider counts it against the
    /// Store's capacity.
    pub bytes: u64,
}
