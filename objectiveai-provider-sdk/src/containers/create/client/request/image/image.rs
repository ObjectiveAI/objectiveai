//! The choice of source.

use serde::{Deserialize, Serialize};

use super::{Client, Registry, Server};

/// Which image, and who is responsible for producing it.
///
/// Three answers to one question — can the provider get these bytes,
/// and if not, who can:
///
/// - [`Registry`](Self::Registry): the caller names a source and the
///   provider fetches it.
/// - [`Server`](Self::Server): the caller names an image and leaves
///   the fetching entirely to the provider.
/// - [`Client`](Self::Client): nobody can fetch it, so the caller
///   supplies it.
///
/// Untagged, with each variant's payload carrying its own `type`
/// constant — the same discipline the agentic loop chunks use. serde
/// has no tag of its own to read, so a source goes on the wire as
/// itself rather than as a wrapper around itself.

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Image {
    /// The caller supplies the image.
    Client(Client),
    /// The provider already has it, or can get it its own way.
    Server(Server),
    /// The caller names a registry reference to pull.
    Registry(Registry),
}
