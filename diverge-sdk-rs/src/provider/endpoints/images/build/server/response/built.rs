//! The image, built.

use serde::{Deserialize, Serialize};

/// The image built: what a listing will report it by, less the name
/// the caller already knows and the time, which is now.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Built {
    /// The manifest digest of the image as the provider holds it,
    /// `<algorithm>:<hex>`: what a container request names.
    pub digest: String,
    /// Its size in BYTES, as the provider counts it against the
    /// store's capacity.
    pub bytes: u64,
}
