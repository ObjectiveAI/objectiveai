//! The image, pushed.

use serde::{Deserialize, Serialize};

use crate::shared::containers::response::Source;

/// The image pushed into the caller's Store: what a listing will
/// report it by, less the description the caller already knows and
/// the time, which is now — and where it came from.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Pushed {
    /// The manifest digest of the image as the provider holds it,
    /// `<algorithm>:<hex>`: the request's, and what a container
    /// request names.
    pub digest: String,
    /// Its size in BYTES, as the provider counts it against the
    /// Store's capacity.
    pub bytes: u64,
    /// Where the provider took it from: a registry it uses, or the
    /// caller, who proved it holds it. Never
    /// [`Store`](Source::Store): a digest the caller owns is `Exists`.
    pub source: Source,
}
