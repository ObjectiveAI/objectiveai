//! An end's one-time key pair.

use p256::elliptic_curve::sec1::ToEncodedPoint as _;
use p256::{PublicKey, SecretKey};
use rand::RngCore as _;
use zeroize::Zeroizing;

use crate::wire::frame::auth::POINT_LEN;
use crate::wire::seal::KEY_LEN;

/// One end's one-time P-256 key pair: made for one handshake, its
/// public half sent, and its private half spent on the one agreement
/// and let go.
pub(super) struct Exchange {
    secret: SecretKey,
}

impl Exchange {
    /// A new pair, from the system's randomness. A draw that is no
    /// scalar of the curve — zero, or not below its order, which
    /// happens about once in 2^32 draws — is drawn again.
    pub(super) fn new() -> Exchange {
        loop {
            let mut seed = Zeroizing::new([0u8; 32]);
            rand::rng().fill_bytes(seed.as_mut());
            if let Ok(secret) = SecretKey::from_slice(seed.as_ref()) {
                return Exchange { secret };
            }
        }
    }

    /// The public half, as it is sent: the uncompressed point.
    pub(super) fn public(&self) -> [u8; POINT_LEN] {
        let point = self.secret.public_key().to_encoded_point(false);
        let mut public = [0u8; POINT_LEN];
        public.copy_from_slice(point.as_bytes());
        public
    }

    /// The secret this pair and `theirs`, the other end's public half,
    /// share: the 32 bytes of the `x` coordinate of the
    /// elliptic-curve Diffie-Hellman point, SEC 1 §3.3.1 — or `None`
    /// where `theirs` is not an uncompressed point of the curve, which
    /// ends the handshake.
    pub(super) fn agree(self, theirs: &[u8; POINT_LEN]) -> Option<[u8; KEY_LEN]> {
        if theirs[0] != 4 {
            return None;
        }
        let theirs = PublicKey::from_sec1_bytes(theirs).ok()?;
        let shared = p256::ecdh::diffie_hellman(self.secret.to_nonzero_scalar(), theirs.as_affine());
        let mut secret = [0u8; KEY_LEN];
        secret.copy_from_slice(shared.raw_secret_bytes());
        Some(secret)
    }
}
