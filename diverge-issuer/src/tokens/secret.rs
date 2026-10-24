//! The opaque secrets: refresh tokens and device codes.

use rand::RngCore as _;
use sha2::{Digest as _, Sha256};

/// A new secret: 32 random bytes as 64 lowercase hex characters.
pub fn mint() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    lower_hex(&bytes)
}

/// The SHA-256 of `secret` as lowercase hex: what the database keeps
/// of it, and what a presented one is looked up by.
pub fn hash(secret: &str) -> String {
    lower_hex(&Sha256::digest(secret.as_bytes()))
}

/// `bytes` as lowercase hex, two characters a byte.
fn lower_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
