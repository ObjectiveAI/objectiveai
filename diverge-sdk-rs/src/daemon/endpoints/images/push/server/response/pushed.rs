//! The image, pushed.

use serde::{Deserialize, Serialize};

use crate::shared::containers::response::Source;

/// The image pushed into the daemon's Store on the provider: the
/// provider's
/// [`Pushed`](crate::provider::endpoints::images::push::server::response::Pushed),
/// as it gave it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Pushed {
    /// The manifest digest of the image as the provider holds it,
    /// `<algorithm>:<hex>`: the request's, and what a template's image
    /// names.
    pub digest: String,
    /// Its size in BYTES, as the provider counts it against the
    /// Store's capacity.
    pub bytes: u64,
    /// Where the provider took it from: a registry it uses, or the
    /// client, who proved it holds it. Never
    /// [`Store`](Source::Store): a digest owned already is `Exists`.
    pub source: Source,
}
