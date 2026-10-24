//! The opaque secrets: refresh tokens and codes.

use diverge_sdk::keyring::Key;

/// A new secret: 32 random bytes as 64 lowercase hex characters, as
/// the keyring mints a key.
pub fn mint() -> Key {
    Key::mint()
}

/// The SHA-256 of `secret` as lowercase hex: what the database keeps
/// of it, and what a presented one is looked up by.
pub fn hash(secret: &str) -> String {
    Key::given(secret.to_string()).hash()
}
