//! One registry a caller may name.

use serde::{Deserialize, Serialize};

use super::Credential;

/// A registry a caller may pull from by naming it in an image
/// reference, and how the provider logs in to it.
///
/// ```yaml
/// - host: docker.io
/// - host: ghcr.io
///   credential:
///     username: bolt
///     password: ghp_…
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registry {
    /// The registry's host, `docker.io`, `ghcr.io`, `registry.example.com:5000`:
    /// what a reference names before its first `/`. No scheme and no
    /// path.
    pub host: String,
    /// What the provider presents to the registry on every pull from
    /// it. Absent means the provider pulls anonymously, which is what
    /// a public image needs. The provider writes every credential of
    /// this section into an auth file of its own, in the form podman
    /// reads, and names that file on every pull; nothing is logged in
    /// through podman itself, and the file is the provider's to keep
    /// private.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential: Option<Credential>,
}

impl Registry {
    /// A registry pulled from anonymously.
    pub fn anonymous(host: &str) -> Self {
        Registry {
            host: host.to_string(),
            credential: None,
        }
    }
}
