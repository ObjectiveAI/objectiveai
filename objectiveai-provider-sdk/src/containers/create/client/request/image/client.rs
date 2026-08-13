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
/// N layer blobs; the manifest is the only small part, so it travels
/// here and the rest is PULLED.
///
/// A provider stands up a registry endpoint of its own and points its
/// container runtime at it. The runtime performs an ordinary pull —
/// asks for the manifest, receives the one sent here, then asks for
/// each blob it does not already hold — and every blob request
/// becomes a channel opened back to this caller.
///
/// # The runtime's cache is the only cache
///
/// A provider does not read the manifest to work out which layers it
/// is missing, and does not keep a blob store to compare against.
/// Container runtimes already index layers by compressed digest and
/// already skip the ones they hold, so letting the runtime do the
/// pulling means that logic is used rather than reimplemented beside
/// it, and there is one cache instead of two that can disagree.
///
/// What falls out is the dedup a caller wants: three variants of one
/// base image send the base once, and an image whose base the
/// provider pulled last week for somebody else sends only the layers
/// that are actually new.
///
/// # Routing
///
/// The scope rides in the repository name the runtime is pointed at.
/// One endpoint serves every creation happening at once, and a
/// request arriving at `/v2/<scope>/…` names the caller it belongs
/// to — so the channel opens against the right connection without a
/// provider keeping any state between requests.
///
/// # Why the blobs have to come from the caller
///
/// A layer is content-addressed, so `sha256:abc…` is the same bytes
/// wherever it lives — and there is still nowhere to ask for it.
/// Registry APIs are repository-scoped (`/v2/<name>/blobs/<digest>`)
/// and a manifest lists digests, not repositories, so a provider has
/// no source to try even for a layer sitting on a public registry at
/// that moment. Either the runtime holds the blob already or this
/// caller sends it.
///
/// # Nothing here is trusted
///
/// The runtime hashes every blob against the digest it asked for, so
/// a caller that sends the wrong bytes fails a comparison rather than
/// a policy check.
///
/// The manifest is checked against nothing, because there is nothing
/// to check it against — it IS the claim. An image's digest is the
/// hash of exactly these bytes, so whatever they hash to is the image
/// the caller asked to create, and a caller cannot lie about which
/// image it sent any more than it can lie about what it typed.
///
/// # Refusing early
///
/// Every descriptor in the manifest carries a size. Summing them
/// gives the exact byte cost before a single blob moves, which is the
/// cheapest possible place for a provider to decline — no transfer
/// wasted, and a real number to decline with.
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
    /// else versions — and a provider does not need one, since it
    /// serves these bytes back verbatim at its registry endpoint and
    /// lets the runtime parse them.
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
