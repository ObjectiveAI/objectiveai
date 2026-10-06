//! What a walk of a volume finds.

use serde::{Deserialize, Serialize};

/// The two things a listing does not say about a volume, as the
/// provider's
/// [`Stat`](crate::provider::endpoints::volumes::stat::server::response::Stat)
/// reports them, as of the moment of the walk: how much of it is used,
/// and the hash of its content.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Stat {
    /// How much of the volume is in use, in bytes, as of the walk.
    pub bytes_used: u64,
    /// The hash of the volume's content, as of the walk: Go's `dirhash`
    /// of its root, `HashDir(root, "", Hash1)`, the `h1:` string,
    /// exactly as the provider defines it and as a directory
    /// [resource](crate::daemon::endpoints::resources)'s id is
    /// computed.
    pub dirhash: String,
}
