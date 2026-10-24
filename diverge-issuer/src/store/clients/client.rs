//! A client, as the store reads it.

use super::{Grant, Kind};

/// One client, whole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Client {
    /// The client's id.
    pub id: String,
    /// Whether it has a secret.
    pub kind: Kind,
    /// The SHA-256 of its secret, as lowercase hex: present exactly
    /// when the client is confidential.
    pub secret_hash: Option<String>,
    /// The grants it may use.
    pub grants: Vec<Grant>,
    /// The audiences its access tokens may be for, the first the one a
    /// request that names none gets.
    pub audiences: Vec<String>,
    /// The URIs the authorization code may be sent to, each matched
    /// exactly.
    pub redirects: Vec<String>,
    /// When it was added, in seconds since the Unix epoch.
    pub made: i64,
}

impl Client {
    /// Whether the client may use `grant`.
    pub fn may(&self, grant: Grant) -> bool {
        self.grants.contains(&grant)
    }
}
