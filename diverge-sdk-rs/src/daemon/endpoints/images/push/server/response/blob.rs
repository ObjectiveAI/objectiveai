//! A blob pulled from the client.

use serde::{Deserialize, Serialize};

/// One blob the provider pulled from the client, through the daemon,
/// and verified: the push's progress. The provider's
/// [`Blob`](crate::provider::endpoints::images::push::server::response::Blob),
/// as it gave it.
///
/// Sent once per blob the provider asked for, after its bytes arrived
/// whole and hashed to its digest, and never for a blob the provider
/// held already or took from a registry. A push that pulls nothing
/// sends none.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Blob {
    /// The blob's digest, `<algorithm>:<hex>`.
    pub digest: String,
    /// Its size in BYTES.
    pub bytes: u64,
}
