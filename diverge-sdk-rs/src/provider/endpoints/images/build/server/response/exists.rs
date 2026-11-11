//! The image, owned already.

use serde::{Deserialize, Serialize};

/// The build produced a digest the caller already owns in its Store:
/// nothing was added, and the image the caller has is the one it
/// asked for.
///
/// Its description is what it was; a build's description is not
/// written over an image owned already. A caller that wants it
/// changed asks an [`edit`](crate::provider::endpoints::images::edit).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Exists {
    /// The manifest digest the build produced, `<algorithm>:<hex>`.
    pub digest: String,
}
