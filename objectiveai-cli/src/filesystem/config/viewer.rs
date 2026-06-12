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

#[derive(
    Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema,
)]
#[schemars(rename = "filesystem.config.ViewerConfig")]
pub struct ViewerConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub address: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub secret: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub signature: Option<String>,
}

impl ViewerConfig {
    pub fn is_empty(&self) -> bool {
        self.address.is_none()
            && self.secret.is_none()
            && self.signature.is_none()
    }

    pub fn is_none(this: &Option<Self>) -> bool {
        this.as_ref().is_none_or(|cfg| cfg.is_empty())
    }

    pub fn get_address(&self) -> Option<&str> {
        self.address.as_deref()
    }

    pub fn set_address(&mut self, value: impl Into<String>) {
        self.address = Some(value.into());
    }


    pub fn get_secret(&self) -> Option<&str> {
        self.secret.as_deref()
    }

    pub fn set_secret(&mut self, value: impl Into<String>) {
        self.secret = Some(value.into());
    }

    pub fn get_signature(&self) -> Option<&str> {
        self.signature.as_deref()
    }

    pub fn set_signature(&mut self, value: impl Into<String>) {
        self.signature = Some(value.into());
    }

    pub fn jq(
        &self,
        filter: &str,
    ) -> Result<Vec<serde_json::Value>, super::super::Error> {
        super::super::run_jq(self, filter)
    }
}
