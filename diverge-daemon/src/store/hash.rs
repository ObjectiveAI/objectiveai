//! The ids that are hashes: a template's — the daemon's own, and a
//! dependency tool template an agent declared — and an identity's,
//! where something is kept by an identity that has no row.

use diverge_sdk::shared::Identity;
use diverge_sdk::shared::canonical;
use serde::Serialize;
use sha2::{Digest as _, Sha256};

/// A template's id: the lowercase hex SHA-256 of its canonical bytes
/// — compact JSON, absent members omitted, every object key sorted at
/// every depth, as [`canonical::bytes`] writes it — so a caller that
/// hashes the same way gets the same id whatever order it spelled
/// the members in. What is handed here is the template's `hashed`
/// projection, the image's references absent, so that one image under
/// different names is one template.
pub fn template_id<T: Serialize>(template: &T) -> Result<String, serde_json::Error> {
    Ok(hex::encode(Sha256::digest(canonical::bytes(template)?)))
}

/// An identity's key: the SHA-256 of its canonical bytes, as
/// [`template_id`] hashes, for an identity that has no row — an
/// administrator the configuration names.
pub fn identity_key(identity: &Identity) -> Result<[u8; 32], serde_json::Error> {
    Ok(Sha256::digest(canonical::bytes(identity)?).into())
}

