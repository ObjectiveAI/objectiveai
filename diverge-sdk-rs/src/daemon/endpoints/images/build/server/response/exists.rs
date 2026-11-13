//! The image, owned already.

use serde::{Deserialize, Serialize};

/// The build produced a digest the daemon already owns in its Store
/// on the provider: nothing was added, the image there is the one
/// asked for, and its description is what it was. The provider's
/// [`Exists`](crate::provider::endpoints::images::build::server::response::Exists),
/// as it gave it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Exists {
    /// The manifest digest the build produced, `<algorithm>:<hex>`.
    pub digest: String,
}
