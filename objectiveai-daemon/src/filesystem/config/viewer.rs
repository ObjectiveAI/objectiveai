use serde::{Deserialize, Serialize};

/// A generated secret/signature pair for viewer authentication.
///
/// The viewer server stores the `secret`. The API client stores the `signature`.
/// The signature is `sha256=<hex of SHA256(secret)>`. The viewer server validates
/// by computing SHA256(secret) and comparing against the incoming header value.
/// Knowing the signature does not reveal the secret (preimage resistance).
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "filesystem.config.ViewerSecretSignaturePair")]
pub struct ViewerSecretSignaturePair {
    /// The secret for the viewer server.
    pub secret: String,
    /// The pre-computed signature for the API client (`sha256=<hex>`).
    pub signature: String,
}

/// Generates a random secret/signature pair for viewer authentication.
///
/// The secret is a random 64-character hex string. The signature is
/// `sha256=<SHA256(secret)>` — a one-way derivation that cannot be reversed.
pub fn generate_viewer_secret_signature_pair() -> ViewerSecretSignaturePair {
    use sha2::{Digest, Sha256};

    let mut bytes = [0u8; 32];
    rand::fill(&mut bytes);
    let secret: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();

    let hash = Sha256::digest(secret.as_bytes());
    let signature = format!(
        "sha256={}",
        hash.iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    );

    ViewerSecretSignaturePair { secret, signature }
}
