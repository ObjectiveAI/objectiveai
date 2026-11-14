//! An end's one-time key pair.

use rand::RngCore as _;
use x25519_dalek::{PublicKey, StaticSecret};

use crate::wire::frame::auth::KEY_LEN;

/// One end's one-time X25519 key pair, RFC 7748: made for one
/// handshake, its public half sent, and its private half spent on the
/// one agreement and let go.
pub(super) struct Exchange {
    secret: StaticSecret,
}

impl Exchange {
    /// A new pair, from the system's randomness.
    pub(super) fn new() -> Exchange {
        let mut seed = [0u8; KEY_LEN];
        rand::rng().fill_bytes(&mut seed);
        Exchange { secret: StaticSecret::from(seed) }
    }

    /// The public half, as it is sent.
    pub(super) fn public(&self) -> [u8; KEY_LEN] {
        PublicKey::from(&self.secret).to_bytes()
    }

    /// The secret this pair and `theirs`, the other end's public half,
    /// share — or `None` where it is all zero bytes, which a key chosen
    /// to make it so yields, and which ends the handshake.
    pub(super) fn agree(self, theirs: &[u8; KEY_LEN]) -> Option<[u8; KEY_LEN]> {
        let shared = self.secret.diffie_hellman(&PublicKey::from(*theirs));
        shared.was_contributory().then(|| shared.to_bytes())
    }
}
