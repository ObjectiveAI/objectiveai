//! What both ends sign, and draw their keys under.

use ed25519_dalek::{Signature, VerifyingKey};
use hkdf::Hkdf;
use sha2::{Digest as _, Sha256};

use crate::wire::frame::auth::{KEY_LEN, SIGNATURE_LEN};
use crate::wire::seal::Seal;

/// What a transcript begins with.
const LABEL: &[u8] = b"diverge handshake";

/// The info the connector's sending key is drawn under.
const CONNECTOR: &[u8] = b"diverge connector";

/// The info the connectee's sending key is drawn under.
const CONNECTEE: &[u8] = b"diverge connectee";

/// Which end signs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Role {
    /// The end that dialled: byte `0`.
    Connector,
    /// The end that accepted: byte `1`.
    Connectee,
}

/// The handshake's transcript — see the [rules](super#the-transcript):
/// the label, both one-time keys, and both tokens, each behind its
/// length.
pub(super) struct Transcript(Vec<u8>);

impl Transcript {
    /// The transcript of the handshake whose one-time keys and tokens
    /// these are.
    pub(super) fn new(connector_key: &[u8; KEY_LEN], connectee_key: &[u8; KEY_LEN], connector_token: &str, connectee_token: &str) -> Transcript {
        let mut bytes = Vec::with_capacity(LABEL.len() + 2 * KEY_LEN + 8 + connector_token.len() + connectee_token.len());
        bytes.extend_from_slice(LABEL);
        bytes.extend_from_slice(connector_key);
        bytes.extend_from_slice(connectee_key);
        for token in [connector_token, connectee_token] {
            // A token longer than a `u32` can say is one no frame
            // carried.
            bytes.extend_from_slice(&u32::try_from(token.len()).unwrap_or(u32::MAX).to_be_bytes());
            bytes.extend_from_slice(token.as_bytes());
        }
        Transcript(bytes)
    }

    /// What `role` signs: the transcript, then its byte.
    pub(super) fn signed(&self, role: Role) -> Vec<u8> {
        let mut message = self.0.clone();
        message.push(match role {
            Role::Connector => 0,
            Role::Connectee => 1,
        });
        message
    }

    /// Whether `signature` is `role`'s over this transcript, by the
    /// Ed25519 public key `key`. A key that is no point of the curve
    /// signs nothing.
    pub(super) fn proven(&self, role: Role, key: &[u8; KEY_LEN], signature: &[u8; SIGNATURE_LEN]) -> bool {
        VerifyingKey::from_bytes(key).is_ok_and(|key| key.verify_strict(&self.signed(role), &Signature::from_bytes(signature)).is_ok())
    }

    /// The two directions the `shared` secret makes under this
    /// transcript: the connector's, then the connectee's, each a
    /// [`Seal`] that has carried nothing.
    pub(super) fn seals(&self, shared: &[u8; KEY_LEN]) -> (Seal, Seal) {
        let salt = Sha256::digest(&self.0);
        let drawn = Hkdf::<Sha256>::new(Some(&salt), shared);
        let seal = |info: &[u8]| {
            let mut key = [0u8; KEY_LEN];
            drawn.expand(info, &mut key).expect("thirty-two bytes is a length HKDF-SHA-256 gives");
            Seal::new(&key)
        };
        (seal(CONNECTOR), seal(CONNECTEE))
    }
}
