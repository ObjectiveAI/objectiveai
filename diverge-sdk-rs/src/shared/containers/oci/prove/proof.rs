//! Computing a proof: the one function both ends use.

use sha2::{Digest as _, Sha256};

/// The proof as it is computed, part by part: `SHA-256(nonce ‖ D)`,
/// with `D` handed over in pieces, in order — see
/// [`prove`](super) for what `D` is and the order of its parts.
///
/// Pieces are concatenated with no separator, so how the bytes are
/// split between calls to [`update`](Self::update) changes nothing:
/// a blob streamed in chunks proves the same as the blob whole.
#[derive(Debug, Clone)]
pub struct Prover {
    hasher: Sha256,
}

impl Prover {
    /// A proof keyed by `nonce`, with nothing of `D` in it yet.
    pub fn new(nonce: &[u8; 32]) -> Prover {
        let mut hasher = Sha256::new();
        hasher.update(nonce);
        Prover { hasher }
    }

    /// The next bytes of `D`.
    pub fn update(&mut self, bytes: &[u8]) {
        self.hasher.update(bytes);
    }

    /// The proof: the 32 bytes of the digest.
    pub fn finish(self) -> [u8; 32] {
        self.hasher.finalize().into()
    }
}

/// The proof over `parts`, in order: the manifest, the platform
/// manifest when there is an index, the config, then each layer.
pub fn proof(nonce: &[u8; 32], parts: impl IntoIterator<Item = impl AsRef<[u8]>>) -> [u8; 32] {
    let mut prover = Prover::new(nonce);
    for part in parts {
        prover.update(part.as_ref());
    }
    prover.finish()
}
