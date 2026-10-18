//! A credential's key: minted once, kept as a hash.

use rand::RngCore as _;
use sha2::{Digest as _, Sha256};

/// How many random bytes a key is.
const KEY_BYTES: usize = 32;

/// A new key: thirty-two random bytes, as sixty-four hex digits. The
/// daemon answers it exactly once, in the response that made the
/// credential, and keeps only its hash.
pub fn mint() -> String {
    let mut bytes = [0u8; KEY_BYTES];
    rand::rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// The SHA-256 of a key, as hex: what the store keeps, and what a
/// presented credential is looked up by. Equal keys hash equal, and a
/// hash tells nobody the key, so a copy of the records is not a copy
/// of every credential.
pub fn hash(key: &str) -> String {
    hex::encode(Sha256::digest(key.as_bytes()))
}
