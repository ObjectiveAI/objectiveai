//! PKCE: RFC 7636, `S256` alone.

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest as _, Sha256};

/// The one method taken.
pub const S256: &str = "S256";

/// Whether `challenge` is an `S256` challenge: the unpadded base64url
/// of a SHA-256, which is 43 characters.
pub fn challenge(challenge: &str) -> bool {
    challenge.len() == 43 && URL_SAFE_NO_PAD.decode(challenge).is_ok_and(|bytes| bytes.len() == 32)
}

/// Whether `verifier` is the one `challenge` was made from. The rules,
/// RFC 7636 §4.1 and §4.6:
///
/// 1. The verifier is 43 to 128 characters, each a letter, a digit,
///    `-`, `.`, `_` or `~`.
/// 2. The unpadded base64url of its SHA-256 is the challenge.
pub fn verify(verifier: &str, challenge: &str) -> bool {
    let unreserved = |byte: u8| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~');
    if !(43..=128).contains(&verifier.len()) || !verifier.bytes().all(unreserved) {
        return false;
    }
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())) == challenge
}
