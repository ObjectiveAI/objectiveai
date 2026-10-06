//! A new resource as a transfer's destination.

use serde::{Deserialize, Serialize};

/// What a transfer that lands in a resource says about it: the
/// description, since the id is the hash of what lands and the kind is
/// the source's. Nothing else is the caller's to choose.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Resource {
    /// What the resource is, in words. Required, as on an upload; the
    /// daemon compares it to nothing and reads it for nothing.
    pub description: String,
}
