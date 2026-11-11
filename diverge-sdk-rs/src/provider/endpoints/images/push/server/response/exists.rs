//! The image, owned already.

use serde::{Deserialize, Serialize};

/// The caller already owns the digest in its Store: nothing was
/// pushed, nothing moved, and the image the caller has is the one it
/// named.
///
/// Its description is what it was; a push's description is not
/// written over an image owned already. A caller that wants it
/// changed asks an [`edit`](crate::provider::endpoints::images::edit).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Exists {
    /// The manifest digest, `<algorithm>:<hex>`, as the request named
    /// it.
    pub digest: String,
}
