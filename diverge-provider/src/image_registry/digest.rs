//! The digests podman speaks, and hashing to them.

use std::fmt;

use sha2::{Digest as _, Sha256, Sha512};

/// A digest as the Distribution API writes one, `<algorithm>:<hex>`,
/// parsed and checked: one of the two algorithms, and lowercase
/// hexadecimal of that algorithm's length. Anything else is not a
/// digest, and a request naming it is a `404`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Digest {
    algorithm: Algorithm,
    hex: String,
}

/// The two algorithms the OCI image specification registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
    Sha256,
    Sha512,
}

impl Algorithm {
    /// The digest's name for it.
    fn name(self) -> &'static str {
        match self {
            Algorithm::Sha256 => "sha256",
            Algorithm::Sha512 => "sha512",
        }
    }

    /// How many hexadecimal characters its digest has.
    fn hex_len(self) -> usize {
        match self {
            // SHA-256: 32 bytes.
            Algorithm::Sha256 => 64,
            // SHA-512: 64 bytes.
            Algorithm::Sha512 => 128,
        }
    }
}

impl Digest {
    /// `<algorithm>:<hex>` read strictly, or `None`.
    pub fn parse(text: &str) -> Option<Self> {
        let (algorithm, hex) = text.split_once(':')?;
        let algorithm = match algorithm {
            "sha256" => Algorithm::Sha256,
            "sha512" => Algorithm::Sha512,
            _ => return None,
        };
        if hex.len() != algorithm.hex_len() || !hex.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')) {
            return None;
        }
        Some(Digest {
            algorithm,
            hex: hex.to_string(),
        })
    }

    /// A hasher for this digest's algorithm, to hash bytes toward it.
    pub fn hasher(&self) -> Hasher {
        match self.algorithm {
            Algorithm::Sha256 => Hasher::Sha256(Sha256::new()),
            Algorithm::Sha512 => Hasher::Sha512(Sha512::new()),
        }
    }

    /// Whether `bytes`, whole, hash to this digest.
    pub fn verifies(&self, bytes: &[u8]) -> bool {
        let mut hasher = self.hasher();
        hasher.update(bytes);
        hasher.finish() == self.hex
    }

    /// Whether a finished hash is this digest.
    pub fn is(&self, hex: &str) -> bool {
        self.hex == hex
    }
}

/// `<algorithm>:<hex>`, as it was parsed from.
impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.algorithm.name(), self.hex)
    }
}

/// A hash in progress, of the algorithm a [`Digest`] named.
pub enum Hasher {
    Sha256(Sha256),
    Sha512(Sha512),
}

impl Hasher {
    /// More of the bytes.
    pub fn update(&mut self, bytes: &[u8]) {
        match self {
            Hasher::Sha256(hasher) => hasher.update(bytes),
            Hasher::Sha512(hasher) => hasher.update(bytes),
        }
    }

    /// The hash of everything so far, as lowercase hexadecimal, which
    /// is what a [`Digest`] compares to.
    pub fn finish(self) -> String {
        match self {
            Hasher::Sha256(hasher) => hex::encode(hasher.finalize()),
            Hasher::Sha512(hasher) => hex::encode(hasher.finalize()),
        }
    }
}
