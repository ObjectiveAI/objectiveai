//! The id that is a hash: a template's.

use serde::Serialize;
use sha2::{Digest as _, Sha256};

/// A template's id: the lowercase hex SHA-256 of its compact JSON —
/// members in the order the type declares them, `type` first, absent
/// members omitted, no whitespace — which is what `serde_json` writes
/// for it, so a caller that hashes the same bytes gets the same id.
pub fn template_id<T: Serialize>(template: &T) -> Result<String, serde_json::Error> {
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(template)?)))
}

