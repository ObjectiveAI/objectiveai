//! One direction of a connection, encrypted.

use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce};
use bytes::Bytes;

/// The length of a session key, in bytes.
pub const KEY_LEN: usize = 32;

/// The length of a nonce, in bytes.
const NONCE_LEN: usize = 12;

/// One direction of a connection past its handshake: the key that
/// direction's messages are encrypted under, and how many it has
/// carried. An end holds two — the one it seals what it sends with,
/// and the one it opens what it receives with — and the other end
/// holds the same two the other way round. The rules:
///
/// 1. **The cipher.** AES-256-GCM, NIST SP 800-38D: the message is the
///    ciphertext of one whole frame, followed by its sixteen-byte tag.
///    No additional data is authenticated.
/// 2. **The nonce.** Twelve bytes: four zero bytes, then the count of
///    messages this direction carried before this one, as a `u64`,
///    big-endian. The first message's count is `0`. The nonce is never
///    sent: each end counts.
/// 3. **In order, once.** A message opens only under the count it was
///    sealed under, so a message dropped, repeated, reordered or
///    altered does not open, and neither does one sealed under the
///    other direction's key.
/// 4. **A message that does not open ends the connection.** Nothing
///    says why, and nothing after it is read.
/// 5. **The count does not wrap.** A direction that has carried 2^64
///    messages carries no more.
pub struct Seal {
    cipher: Aes256Gcm,
    count: u64,
}

/// A message that could not be sealed, or did not open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Broken;

impl Seal {
    /// The direction whose key is `key`, having carried nothing.
    pub fn new(key: &[u8; KEY_LEN]) -> Seal {
        Seal { cipher: Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key)), count: 0 }
    }

    /// The nonce of the next message, the count moved on past it —
    /// rules 2 and 5.
    fn nonce(&mut self) -> Result<[u8; NONCE_LEN], Broken> {
        let mut nonce = [0u8; NONCE_LEN];
        nonce[4..].copy_from_slice(&self.count.to_be_bytes());
        self.count = self.count.checked_add(1).ok_or(Broken)?;
        Ok(nonce)
    }

    /// `frame`, sealed as this direction's next message.
    pub fn seal(&mut self, frame: &[u8]) -> Result<Bytes, Broken> {
        let nonce = self.nonce()?;
        self.cipher.encrypt(Nonce::from_slice(&nonce), frame).map(Bytes::from).map_err(|_| Broken)
    }

    /// The frame `message` holds, opened as this direction's next
    /// message — rules 3 and 4.
    pub fn open(&mut self, message: &[u8]) -> Result<Bytes, Broken> {
        let nonce = self.nonce()?;
        self.cipher.decrypt(Nonce::from_slice(&nonce), message).map(Bytes::from).map_err(|_| Broken)
    }
}

impl std::fmt::Debug for Seal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Seal(<hidden>)")
    }
}
