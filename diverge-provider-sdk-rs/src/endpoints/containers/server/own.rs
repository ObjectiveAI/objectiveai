//! What the provider asks a caller for on its own account.

use crate::shared::containers::authorize::request::Authorize;
use crate::shared::containers::tools::Tool;

/// The provider's own asks, before the container's: what a run
/// handler needs from the caller that no container asked for. Each
/// family carries these as its own frame type — see
/// [`Runs::Ask`](super::family::Runs::Ask), which every one converts
/// into — so the machinery names them once, here, and a family says
/// how they are spelled.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Own<'a> {
    /// A manifest of an image the caller holds, by digest.
    OciManifest(&'a str),
    /// A blob of such an image, by digest.
    OciBlob(&'a str),
    /// Whether the caller holds an image, by name and digest.
    OciHas {
        /// The repository path.
        name: &'a str,
        /// The manifest digest.
        digest: &'a str,
    },
    /// Whether a connector may attach.
    Authorize(Authorize),
    /// The tools the container declared, to deploy.
    Tools(&'a [Tool]),
    /// This end's half of a database connection, by the id it minted.
    Postgres(u32),
}
