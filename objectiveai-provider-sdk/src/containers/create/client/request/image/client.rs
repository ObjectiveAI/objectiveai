//! The caller supplies the image.

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

/// The caller supplies the image.
///
/// For images that exist nowhere a provider can reach — built
/// locally, never pushed, and carrying a digest no registry has ever
/// heard of.
///
/// # How the bytes get there
///
/// Not in this frame. An image is a manifest plus a config blob plus
/// N layer blobs, and the manifest is the only small part — so the
/// manifest travels here and the rest is FETCHED, by the provider,
/// over channels it opens inside this scope.
///
/// The provider reads the manifest, diffs the digests it names
/// against what it already holds, and asks for what is missing and
/// nothing else. A caller pushing three variants of one base image
/// sends the base once and two thin layers after; a caller whose base
/// the provider already pulled for somebody else sends only its own
/// work.
///
/// This is also why blobs cannot come from anywhere but the caller.
/// A layer is content-addressed, so `sha256:abc…` is the same bytes
/// wherever it lives — but registry APIs are repository-scoped
/// (`/v2/<name>/blobs/<digest>`), and a manifest lists digests, not
/// repositories. A provider holding one has nowhere to ask, even for
/// a layer sitting on a public registry at that moment. Either it has
/// the blob already or the caller sends it.
///
/// # Nothing here is trusted
///
/// Every piece is content-addressed and every piece is checked. The
/// provider hashes [`manifest`](Self::manifest) and confirms it is
/// the digest it was told, before a single blob moves; then hashes
/// each blob against the digest it asked for. A caller that sends the
/// wrong bytes fails a comparison, not a policy check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Client {
    /// The discriminator.
    pub r#type: ClientType,
    /// The image manifest, verbatim.
    ///
    /// Raw, and never re-serialized. The image's digest is the hash of
    /// EXACTLY these bytes, so parsing and re-emitting them — even
    /// into equivalent JSON — produces a different digest and an image
    /// that verifies as nothing. A [`RawValue`] nests into this frame
    /// with no re-encoding and comes out the far side byte-identical.
    ///
    /// It is OCI's schema, not this one's. Describing it here would
    /// mean maintaining a second definition of a document somebody
    /// else versions.
    pub manifest: Box<RawValue>,
}

impl PartialEq for Client {
    fn eq(&self, other: &Self) -> bool {
        self.r#type == other.r#type
            && self.manifest.get() == other.manifest.get()
    }
}

impl Eq for Client {}

/// [`Client`]'s discriminator.
///
/// One variant, and part of why [`Image`](super::Image) can be
/// untagged: no other source can produce this value.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub enum ClientType {
    /// Always this.
    #[serde(rename = "client")]
    #[default]
    Client,
}
