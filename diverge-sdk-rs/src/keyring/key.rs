//! A key, held so that it cannot leave.

use std::fmt;

use rand::RngCore;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

/// The random bytes a minted key is made of, before hex.
const BYTES: usize = 32;

/// One key. No `Serialize`, no `Deserialize`, no `Display`: see
/// [the module](super). Zeroed when it drops.
#[derive(Clone)]
pub struct Key(Zeroizing<String>);

impl Key {
    /// A new key: 32 random bytes, as 64 lowercase hex digits.
    pub fn mint() -> Key {
        let mut bytes = Zeroizing::new([0u8; BYTES]);
        rand::rng().fill_bytes(bytes.as_mut());
        Key(Zeroizing::new(hex::encode(bytes.as_ref())))
    }

    /// A key someone was given, as its text.
    pub fn given(key: String) -> Key {
        Key(Zeroizing::new(key))
    }

    /// The key's text. Called only to present the key on a wire,
    /// compare it with one presented, or hash it.
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// The key's SHA-256, as 64 lowercase hex digits: what a server
    /// that judges keys keeps in place of the key.
    pub fn hash(&self) -> String {
        hex::encode(Sha256::digest(self.0.as_bytes()))
    }
}

impl fmt::Debug for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Key(<hidden>)")
    }
}
