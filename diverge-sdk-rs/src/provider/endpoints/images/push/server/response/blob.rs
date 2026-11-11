//! A blob pulled from the caller.

use serde::{Deserialize, Serialize};

/// One blob the provider pulled from the caller and verified: the
/// push's progress.
///
/// Sent once per blob the provider asked the caller for, after its
/// bytes arrived whole and hashed to its digest, and never for a blob
/// the provider held already or took from a registry. A push that
/// pulls nothing sends none.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Blob {
    /// The blob's digest, `<algorithm>:<hex>`.
    pub digest: String,
    /// Its size in BYTES.
    pub bytes: u64,
}
