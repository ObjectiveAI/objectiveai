//! Asking for the proof that an image is held.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Error;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// The platform and the nonce: everything the proof is computed over
/// that the caller does not already hold. The image is not among
/// them: it is the one the scope carrying the ask names — a run's
/// image, a push's image, a read scope's image — and an ask names no
/// other.
///
/// # On the wire
///
/// Its JSON, as the other three asks', with the nonce as 64 lowercase
/// hexadecimal characters:
///
/// ```json
/// {"platform":"sha256:<hex>","nonce":"<64 hex>"}
/// ```
///
/// `platform` is absent when it is `None`. A nonce that is not exactly
/// 64 hexadecimal characters is a request that does not parse.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Request {
    /// When the digest of the scope's image names an index, the digest
    /// of the one manifest in it whose image is meant,
    /// `<algorithm>:<hex>`; `None` when it names an image manifest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    /// The nonce: 32 bytes, fresh, from a cryptographic random source,
    /// for this one check. See [`prove`](super::super).
    #[serde(serialize_with = "nonce_hex", deserialize_with = "nonce_from_hex")]
    pub nonce: [u8; 32],
}

/// The nonce as 64 lowercase hexadecimal characters.
fn nonce_hex<S: Serializer>(nonce: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error> {
    let mut text = String::with_capacity(64);
    for byte in nonce {
        text.push(char::from_digit(u32::from(byte >> 4), 16).expect("a nibble is a hex digit"));
        text.push(char::from_digit(u32::from(byte & 0x0f), 16).expect("a nibble is a hex digit"));
    }
    serializer.serialize_str(&text)
}

/// The nonce from 64 hexadecimal characters, of either case.
fn nonce_from_hex<'de, D: Deserializer<'de>>(deserializer: D) -> Result<[u8; 32], D::Error> {
    let text = String::deserialize(deserializer)?;
    let digits = text.as_bytes();
    if digits.len() != 64 {
        return Err(serde::de::Error::custom("a nonce is 64 hexadecimal characters"));
    }
    let mut nonce = [0u8; 32];
    for (byte, pair) in nonce.iter_mut().zip(digits.chunks_exact(2)) {
        let high = char::from(pair[0]).to_digit(16);
        let low = char::from(pair[1]).to_digit(16);
        let (Some(high), Some(low)) = (high, low) else {
            return Err(serde::de::Error::custom("a nonce is 64 hexadecimal characters"));
        };
        *byte = (high * 16 + low) as u8;
    }
    Ok(nonce)
}

/// Its JSON, and nothing in front of it. The tag that says which
/// request this is belongs to whichever frame carries it.
impl Encode for Request {
    /// The ordinary JSON failure.
    type Error = Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Error> {
        serde_json::to_writer(out, self)
    }
}

impl Decode<'_> for Request {
    /// The ordinary JSON failure.
    type Error = Error;

    fn decode(bytes: &[u8]) -> Result<Self, Error> {
        serde_json::from_slice(bytes)
    }
}
