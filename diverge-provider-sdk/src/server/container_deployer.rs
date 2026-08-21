//! Putting a container somewhere.

use std::future::Future;

use super::client_registry::ClientRegistry;
use super::deployment::Deployment;
use crate::shared::error::Error;

/// What runs a container for a provider.
///
/// The first thing this crate asks a provider to supply, and the one
/// every container endpoint needs. An
/// [`agentic_loop`](crate::endpoints::agentic_loop::run) runs an agent
/// in a container, a
/// [`laboratory`](crate::endpoints::laboratories::run) is one an agent
/// works inside, and an
/// [`mcp_plugin`](crate::endpoints::mcp_plugin::run) is one that serves
/// tools — three endpoints that differ in what goes IN a container and
/// not in how one is deployed.
///
/// So this is generic and lives here rather than under any of them,
/// beside [`Session`](super::session::Session) and for the same reason:
/// it is about running things, not about what was asked.
///
/// # Three methods, one per source
///
/// [`Image`](crate::shared::container::request::Image) has three
/// variants and this has three methods, named for them. Which is not
/// bookkeeping — it is what makes the fourth argument possible.
///
/// A caller-served image is the one case where the provider has to ask
/// somebody for the bytes, so [`client`](Self::client) is handed a
/// [`ClientRegistry`] and the other two are not. One method taking an
/// [`Image`](crate::shared::container::request::Image) would have to
/// carry that as an [`Option`], `Some` exactly when the variant is
/// `Client` — a correlation nothing would enforce and every
/// implementation would have to be trusted to respect.
///
/// Three methods make it structural. The argument exists where it
/// applies and does not exist where it does not, and no implementation
/// has to handle a combination that cannot happen.
///
/// # What a container IS is the provider's
///
/// [`Container`](Self::Container) is unbounded on purpose. A provider
/// hands back whatever it holds a running container by — a process
/// handle, a name, an id its runtime minted, a struct with all three —
/// and this crate does not look inside.
///
/// What can be DONE with one is a separate trait and is not written.
/// Naming the type without bounding it is what lets this land without
/// guessing that surface, the same state
/// [`McpProxy`](crate::client::mcp_proxy::McpProxy) sat in before
/// anything used it.
///
/// # Failure is the provider's vocabulary
///
/// [`Error`] is the opaque one every endpoint's failure variant already
/// carries. A provider says what went wrong in whatever shape it likes
/// and the SDK relays it — which is what would happen to a typed error
/// anyway, one conversion later.
///
/// There is no separate "refused" and "broke". A container that is not
/// running is a container that is not running, and the caller's
/// remedies are the same either way.
pub trait ContainerDeployer: Send + Sync {
    /// A running container, however this provider holds one.
    ///
    /// [`Send`] and `'static` because it outlives the deploy that made
    /// it and will be used from wherever the scope is being served,
    /// which is not where it was built.
    ///
    /// Nothing else is required of it, because nothing here does
    /// anything with it yet.
    type Container: Send + 'static;

    /// Deploy from an image the CALLER serves.
    ///
    /// For images that exist nowhere a provider can reach — built
    /// locally, never pushed, carrying a digest no registry has heard
    /// of. The provider stands up a registry endpoint, points its
    /// runtime at it, and relays through `registry`.
    ///
    /// # What the provider does not do
    ///
    /// Parse a manifest, diff layer digests, or decide what a blob is.
    /// A runtime already indexes layers by digest and already skips the
    /// ones it holds, so letting it pull means that logic is USED
    /// rather than reimplemented beside it — one cache, and no second
    /// one to disagree with it.
    ///
    /// # The name is a path fragment, and is not checked
    ///
    /// It lands in a URL by concatenation. A `..` in it walks out of
    /// the scope segment and into another caller's namespace, so an
    /// implementation normalizes or refuses before concatenating.
    /// Nothing upstream of here does it.
    ///
    /// The digest needs no such care: a runtime recomputes the hash on
    /// pull and rejects a mismatch, so a caller serving the wrong bytes
    /// under the right name fails at the runtime rather than quietly
    /// succeeding.
    ///
    /// # The future is [`Send`]
    ///
    /// Because a provider deploys for several callers at once. It is
    /// spelled out rather than left to `async fn`, which promises
    /// nothing about the future it returns.
    fn client(
        &self,
        deployment: &Deployment,
        name: &str,
        digest: &str,
        registry: ClientRegistry<'_>,
    ) -> impl Future<Output = Result<Self::Container, Error>> + Send;

    /// Deploy from an image the PROVIDER produces.
    ///
    /// Its own mirror, a pull-through cache, a private registry it
    /// holds credentials for, or something already on disk. Where it
    /// comes from is not a caller's business and a caller is not told.
    ///
    /// Which is what makes proprietary images expressible: an
    /// implementation serves one no public registry carries, and a
    /// caller asks for it without ever being able to fetch it itself.
    ///
    /// The pair is the one
    /// [`images::check`](crate::endpoints::images::check) asks about, so
    /// a check that came back available names an image this can be
    /// handed with nothing to translate.
    fn server(
        &self,
        deployment: &Deployment,
        name: &str,
        digest: &str,
    ) -> impl Future<Output = Result<Self::Container, Error>> + Send;

    /// Deploy from wherever the CALLER says.
    ///
    /// The one case where the caller chooses the source. `reference` is
    /// whatever a container runtime accepts —
    /// `ghcr.io/org/image@sha256:…`, `docker.io/library/ubuntu:22.04` —
    /// host, repository, and tag or digest, in the form the runtime
    /// already parses.
    ///
    /// # It is the one that need not be pinned
    ///
    /// A caller writing `ubuntu:22.04` is asking to track it, the way a
    /// loose version constraint tracks a dependency. One that wants the
    /// guarantee writes a digest into the reference and gets it. The
    /// other two variants are pinned by construction; this is a choice
    /// the caller made.
    ///
    /// # Which registries are reachable is policy
    ///
    /// An implementation's to set and to enforce, and nothing in this
    /// protocol expresses it. A caller naming a host the provider will
    /// not go to learns so by being refused, which is an [`Error`] like
    /// any other.
    fn registry(
        &self,
        deployment: &Deployment,
        reference: &str,
    ) -> impl Future<Output = Result<Self::Container, Error>> + Send;
}
