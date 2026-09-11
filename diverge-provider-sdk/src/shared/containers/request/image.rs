//! What image a container is made from, and who supplies it.

use serde::{Deserialize, Serialize};

/// The image, named the way whoever supplies it can be asked for it.
///
/// One question — can the provider get these bytes, and if not, who
/// can — and the answer decides how the image is named, which is why
/// this is one field rather than a source beside a string that means
/// something different depending on it.
///
/// # Two of the three are pinned, and the third is a choice
///
/// [`Client`](Self::Client) and [`Server`](Self::Server) carry a
/// repository name and a manifest digest, the same pair
/// [`images::check`](crate::endpoints::images::check::client::request::Frame)
/// asks about — so a check that came back available names an image a
/// run can ask for, with nothing to translate between them.
///
/// A digest cannot be repointed at different content, so those two
/// mean the same bytes every time. [`Registry`](Self::Registry) need
/// not, and that is the point: a caller writing `ubuntu:22.04` is
/// asking to track it, exactly as a loose version constraint on a
/// Python requirement is. One that wants the guarantee writes a digest
/// into the reference and gets it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Image {
    /// The caller holds it.
    ///
    /// For images that exist nowhere a provider can reach — built
    /// locally, never pushed, carrying a digest no registry has heard
    /// of.
    ///
    /// The provider runs a registry of its own and its runtime pulls
    /// from that; what the registry does not hold, the provider asks
    /// the caller for by digest — a manifest, a blob — over
    /// [`oci`](crate::shared::containers::oci). The caller needs no
    /// registry and no HTTP: it needs the image's manifest and blobs
    /// in a store keyed by digest, which is what an image is once it
    /// has been saved anywhere, and it answers two kinds of fetch.
    ///
    /// The runtime never learns the registry is a proxy, and every
    /// header it relies on — `Content-Type`, `Content-Length`,
    /// `Range`, `Docker-Content-Digest` — is the provider's registry
    /// answering from its store. A runtime already indexes layers by
    /// digest and skips the ones it holds, so letting it pull means
    /// that logic is USED rather than reimplemented beside it.
    Client {
        /// The repository path — `library/nginx`, `myorg/myimage`.
        ///
        /// # It lands in a URL path
        ///
        /// The provider builds its pull reference by concatenation —
        /// its registry's address, a repository segment, then this —
        /// with no parsing. Which makes it a path fragment wearing the costume
        /// of a name: a `..` in it walks out of the scope segment and
        /// into another caller's namespace, so a provider normalizes
        /// or refuses before concatenating. The field cannot enforce
        /// that and does not pretend to.
        name: String,
        /// The manifest digest, `<algorithm>:<hex>`.
        ///
        /// What actually identifies the image, and the reason nothing
        /// here has to trust the name: the provider hashes what the
        /// caller sent before storing it, and the runtime hashes again
        /// on pull, so a caller holding the wrong bytes under the
        /// right digest fails before anything runs.
        digest: String,
    },
    /// The provider produces it, however it likes.
    ///
    /// Its own mirror, a pull-through cache, a private registry it
    /// holds credentials for, or something already on disk. A caller
    /// does not know and is not told.
    ///
    /// Which is what makes proprietary images expressible: a provider
    /// serves an image no public registry carries, and a caller asks
    /// for it, without the caller ever being able to fetch it itself.
    ///
    /// Ask [`images::check`](crate::endpoints::images::check) first if
    /// the answer matters before the container does. It takes this
    /// same pair, so what a check said yes to is what a run names.
    Server {
        /// The repository path — `library/nginx`, `myorg/myimage`.
        ///
        /// Kept alongside the digest because a digest alone is not
        /// resolvable: every registry API is repository-scoped, and
        /// there is no lookup from a digest to wherever it lives.
        ///
        /// No host. Where a provider gets the image is the provider's
        /// business, and a caller naming a source it cannot reach
        /// would be asserting something it has no standing to assert.
        name: String,
        /// The manifest digest, `<algorithm>:<hex>`.
        ///
        /// What actually identifies the image. Any registry serving
        /// these bytes serves the same image, which is what lets the
        /// provider choose where to get them.
        digest: String,
    },
    /// The provider pulls from where the caller says.
    ///
    /// The one case where the CALLER chooses the source, for public
    /// images where it knows what it wants and the provider has no
    /// opinion.
    ///
    /// Which makes the reference a host a caller picked, and the
    /// provider connects there and runs what it finds. Which
    /// registries are reachable is a provider's policy to set and
    /// enforce, and nothing here can express that policy — a caller
    /// learns it by being refused.
    Registry {
        /// Whatever a container runtime accepts —
        /// `ghcr.io/org/image@sha256:…`, `docker.io/library/ubuntu:22.04`.
        ///
        /// One string rather than a name and a digest, because the
        /// other two variants split them for a reason that does not
        /// apply here. There the pair exists so a provider can go to a
        /// source the caller did not name; here the caller named the
        /// source, and a reference is how a source is named — host,
        /// repository, and tag or digest, in the form the runtime
        /// already parses.
        reference: String,
    },
}
