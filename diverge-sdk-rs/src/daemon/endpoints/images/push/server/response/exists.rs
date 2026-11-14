//! The image, owned already.

use serde::{Deserialize, Serialize};

/// The daemon already owns the digest in its Store on the provider:
/// nothing was pushed, nothing moved, and the image there is the one
/// named. Its description is what it was. The provider's
/// [`Exists`](crate::provider::endpoints::images::push::server::response::Exists),
/// as it gave it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Exists {
    /// The manifest digest, `<algorithm>:<hex>`, as the request named
    /// it.
    pub digest: String,
}
