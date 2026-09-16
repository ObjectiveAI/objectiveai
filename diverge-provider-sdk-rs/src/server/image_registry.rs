//! The registry a runtime pulls a caller-held image from.

use std::future::Future;
use std::net::SocketAddr;

use super::image_source::ImageSource;

/// The provider's own OCI registry: the read side of the Distribution
/// API, on its loopback, that its runtime pulls from when a run names
/// an [`Image::Client`](crate::shared::containers::request::Image::Client).
///
/// What a caller holds is a store of manifests and blobs by digest,
/// and no registry. What a runtime wants is a registry, and no
/// channel to a caller. This trait is the provider standing between
/// them: a run handler tells it, before the deploy, that `repository`
/// is to be served from an [`ImageSource`] — the run scope's channels
/// to the caller — and the deployer then points the runtime at
/// `<address>/<repository>/<name>@<digest>` and pulls as from any
/// registry. See [`oci`](crate::shared::containers::oci) for the
/// pull, request by request, and what each needs from the caller.
///
/// # What an implementation does
///
/// Listens at [`address`](Self::address) and answers what its runtime
/// sends to pull by digest: `GET /v2/`, a manifest by digest, a blob
/// by digest, `GET` or `HEAD`. What it does not hold, under a
/// repository it is serving, it asks the source for —
/// [`ImageSource::manifest`] or [`ImageSource::blob`] — and serves
/// only if the bytes hash to the digest, whether it holds them or
/// streams them through. A source that answers `None`, and bytes
/// that do not hash, are a pull that fails, which is what makes the
/// deploy fail. Whether it keeps anything, and how it answers a
/// `Range`, are its own: the one requirement is that the runtime
/// gets the image the digest names, or nothing.
///
/// # A repository is a run
///
/// Named by the handler, unique per run, released when the run ends.
/// Between [`serve`](Self::serve) and [`release`](Self::release) the
/// registry may ask the source; after, nothing asks.
///
/// # Why this is not in the crate
///
/// Because the crate serves no HTTP, by design: it names both socket
/// types and speaks neither, and a registry is an HTTP server with
/// opinions about caching, ranges and where bytes live. Every one of
/// those is the provider's. What the crate supplies is the source —
/// the one thing only the run scope can be.
pub trait ImageRegistry: Send + Sync {
    /// Why a repository could not be served: the registry is not
    /// running, the name is taken. The provider's own; it reaches the
    /// caller as the run's [`Error`](crate::shared::error::Error).
    type Error: Send + 'static;

    /// Where the runtime pulls from: the registry's listening address,
    /// as the deployer's
    /// [`client`](super::container_deployer::ContainerDeployer::client)
    /// is handed it.
    fn address(&self) -> SocketAddr;

    /// Serve `repository` from `source`, from now until
    /// [`release`](Self::release). Resolves when the registry will
    /// answer for it: the deploy that pulls it comes next.
    fn serve(&self, repository: &str, source: ImageSource) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Stop serving `repository`: the run is over, and its source
    /// with it. Idempotent; a name never served is nothing to do.
    fn release(&self, repository: &str) -> impl Future<Output = ()> + Send;
}
