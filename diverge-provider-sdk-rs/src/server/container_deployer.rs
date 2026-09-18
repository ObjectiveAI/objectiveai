//! Putting a container somewhere.

use std::future::Future;

use super::caller::Caller;
use super::container::Container;
use super::deployment::Deployment;

/// What runs a container for a provider.
///
/// The first thing this crate asks a provider to supply, and the one
/// every [`containers`](crate::endpoints::containers) scope needs. An
/// agent container runs an agent and a tool container serves tools —
/// two families that differ in what goes IN a container and not in
/// how one is deployed.
///
/// So this is generic and lives here rather than under any of them,
/// beside [`Session`](super::session::Session) and for the same reason:
/// it is about running things, not about what was asked.
///
/// # The container is RUNNING when a method returns
///
/// Not created, not scheduled, not queued behind a pull — running. An
/// implementation that has more to do does it before it returns
/// [`Ok`], and one that cannot finish returns
/// [`Err`](Self::Error).
///
/// This protocol has nothing that means "starting". A scope either
/// answers or it fails, and the first thing a handler does with a
/// container is dial its proxy. There is no state between "asked for"
/// and "usable" for a caller to wait in.
///
/// # Which is why there is no create-then-start
///
/// Splitting the deploy would let a provider write files into a
/// container before anything ran in it, which is what
/// `podman create` followed by `podman cp` does. It is also a shape
/// most backends do not have: an instance on a cloud provider boots
/// from an image with no idle filesystem to write into, and a pod's
/// filesystem is not reachable until something is running in it.
///
/// So a two-step deploy would put one container runtime's lifecycle in
/// a trait meant for any of them, and every implementation that is not
/// that runtime would fake the half it lacks.
///
/// Nothing needs it. Getting data in ahead of time is
/// [`mounts`](Deployment::mounts), which every backend expresses —
/// a bind mount, an attached disk, a volume. And a caller's own writes
/// are channel requests inside the scope, which only exists once the
/// container does, so the protocol has no pre-start write and never
/// had one. A laboratory's file transfers happen while it runs.
///
/// # And its proxy is LISTENING when a method returns
///
/// Deploying is also injecting the
/// [`container_proxy_endpoints`](crate::container_proxy_endpoints) into the container and
/// starting it — the image never carries it, and nothing else in this
/// crate can put it there. By the time this is [`Ok`], the proxy
/// accepts a connection at the container's
/// [`address`](Container::address). Not "will shortly" — a handler's
/// first act after a deploy is to dial it, and it dials once.
///
/// One port, always the same one: the proxy's
/// [`OUTSIDE_PORT`](crate::container_proxy_endpoints::OUTSIDE_PORT). A
/// [`Deployment`] names none, because there is nothing to choose. How
/// a provider makes it reachable — published to a loopback port it
/// picked, routed to a bridge address, something a cloud runtime does
/// that resembles neither — is its own, and on the runtimes that need
/// publishing it happens at creation, which is one more reason the
/// port is fixed rather than asked for later. The entrypoint's port is
/// behind the proxy, on the loopback inside, and is never published.
///
/// # Why the waiting is not this protocol's
///
/// Because nothing here can do it. Knowing that a process inside a
/// container has bound a socket means knowing something about the
/// runtime: a health check, a readiness probe, a socket poll, a
/// notification the orchestrator already has. A provider has all of
/// that and this crate has none of it, so a wait written here would be
/// a retry loop guessing at what a provider could simply have known.
///
/// It would also have to guess how long, and this protocol has no
/// timeouts anywhere. The choice would be between giving up early on a
/// container that was coming and waiting forever on one that never
/// was — which is exactly the decision a provider is equipped to make
/// and a caller is not.
///
/// So it is an assumption, stated: an implementation that has more to
/// wait for waits before it returns, the same way it does for the
/// container itself.
///
/// # An entrypoint that never binds is a different thing
///
/// And it is still not detectable here. An image that binds nothing on
/// its entrypoint's port is a container that came up perfectly, whose
/// proxy answers and has nothing behind it, and no amount of waiting
/// turns that into an answer — which is why it surfaces as an exchange
/// the proxy cannot serve.
///
/// The distinction is between not YET and not EVER. This promises the
/// first is over for the proxy; nothing can promise the second away
/// for the image.
///
/// # One method, and the source is the provider's
///
/// An [`Image`](crate::shared::containers::request::Image) is a name
/// and a digest, and says nothing about where the bytes come from.
/// That is the implementation's decision, made from the pair alone:
/// its own store, a registry it uses, the caller through the
/// [`Caller`] it is handed — in whatever order, by whatever policy.
/// What the protocol asserts is only that the container runs the
/// image the digest names, and a runtime that hashes what it pulls
/// makes that so wherever the bytes were found.
///
/// # What a container IS is the provider's
///
/// A provider hands back whatever it holds a running container by — a
/// process handle, a name, an id its runtime minted, a struct with all
/// three — and this crate does not look inside.
///
/// What it must be ABLE to do is [`Container`], which is two methods:
/// saying where its proxy answers, and stopping. Everything else a
/// container is asked for is spoken to the proxy, and relaying does
/// not need the container to have a method for it.
///
/// # Every method is told whose it is
///
/// `client_identity` is whatever authenticated the connection the
/// request arrived on — the same opaque string a
/// [`mount`](super::mount::Mount) carries and a
/// [`VolumeManager`](super::volume_manager::VolumeManager) takes. This
/// crate never mints one, parses one, or compares two.
///
/// It is here because deploying is the most consequential thing a
/// provider does on somebody's behalf, and every question worth asking
/// about it needs to know whose behalf. What a caller may spend, which
/// registries its credentials reach, where its containers are placed,
/// what to bill, what to write down afterwards — none of it is
/// answerable from a [`Deployment`], and all of it is a provider's to
/// decide rather than this protocol's.
///
/// Which is why it is an argument and not a constructor parameter: one
/// deployer serves every caller on every connection, the same way one
/// [`VolumeManager`](super::volume_manager::VolumeManager) does.
///
/// # Failure is the provider's too
///
/// [`Error`](Self::Error) is an associated type for the same reason
/// [`Container`](Self::Container) is: what goes wrong deploying a
/// container is a runtime's business, a kernel's, a registry's, and
/// this crate knows none of them.
///
/// It would have been easy to require
/// [`shared::error::Error`](crate::shared::error::Error) — the opaque
/// one every endpoint's failure variant carries — since that is where a
/// failure ends up. That would have made an implementation build a
/// `serde_json::Value` at the point it has a real error in hand, which
/// is the worst place to lose it: the type that knows most about what
/// happened, thrown away first.
///
/// So a provider returns its own, and getting one onto the wire is the
/// handler's problem — see below.
///
/// There is no separate "refused" and "broke". A container that is not
/// running is a container that is not running, and a caller's remedies
/// are the same either way.
pub trait ContainerDeployer: Send + Sync {
    /// A running container, however this provider holds one.
    ///
    /// `'static` because it outlives the deploy that made it. [`Send`]
    /// and [`Sync`] come from [`Container`] itself, which needs them
    /// for the same reason: a scope is served by more than one task and
    /// a container is reached from any of them.
    type Container: Container + 'static;

    /// Why a container is not running.
    ///
    /// Whatever the provider's own failure type is. A runtime's exit
    /// status, a registry's refusal, a kernel saying no — this crate
    /// does not name any of them and does not convert one.
    ///
    /// [`Send`] and `'static` for the same reasons
    /// [`Container`](Self::Container) is: the future returning it is
    /// [`Send`], so its output has to be, and it outlives the deploy
    /// that produced it.
    ///
    /// # Something will have to render one
    ///
    /// A failure reaches a caller as
    /// [`shared::error::Error`](crate::shared::error::Error), which is
    /// one JSON value and nothing else. So a handler that reports a
    /// deploy failure needs a way to turn one of these into one, and
    /// this trait deliberately does not say how.
    ///
    /// The bound lives on [`handle`](super::handle::handle), which asks
    /// `Into<Error>` of it, rather than here: an [`Into`] bound on the
    /// trait would be this crate's error type back in the signature
    /// under a different name, which is the thing an associated type
    /// was for.
    type Error: Send + 'static;

    /// Deploy the image `name` at `digest`, from wherever the
    /// implementation gets it.
    ///
    /// Its own store, a mirror, a registry it holds credentials for,
    /// or the caller: `caller` says whether the caller holds the
    /// image and where the provider's registry serves it when it
    /// does, and an implementation that takes the image from there
    /// points its runtime at `<registry>/<repository>/<name>@<digest>`
    /// and pulls as from any registry. Which sources it tries, and in
    /// what order, is its own; a pair it can get from none of them is
    /// an [`Error`](Self::Error) like any other. The pair is the one
    /// [`images::check`](crate::endpoints::images::check) asks about,
    /// so a check that came back available names an image this can
    /// be handed with nothing to translate.
    ///
    /// # What the implementation does not do
    ///
    /// Parse a manifest, diff layer digests, or decide what a blob is.
    /// A runtime already indexes layers by digest and already skips the
    /// ones it holds, so letting it pull means that logic is USED
    /// rather than reimplemented beside it — one cache, and no second
    /// one to disagree with it.
    ///
    /// # The name is a path fragment, and is not checked
    ///
    /// It lands in a reference by concatenation. A `..` in it walks
    /// out of the repository segment and into another caller's
    /// namespace, so an implementation refuses a `name` that is not a
    /// repository path before concatenating, and normalizes nothing.
    /// Nothing upstream of here does it.
    ///
    /// The digest needs no such care: the provider's registry hashes
    /// what the caller sent before serving it, and the runtime hashes
    /// again on pull, so a source holding the wrong bytes under the
    /// right digest fails before anything runs.
    ///
    /// # The future is [`Send`]
    ///
    /// Because a provider deploys for several callers at once. It is
    /// spelled out rather than left to `async fn`, which promises
    /// nothing about the future it returns.
    fn deploy(
        &self,
        client_identity: &str,
        deployment: &Deployment,
        name: &str,
        digest: &str,
        caller: &Caller,
    ) -> impl Future<Output = Result<Self::Container, Self::Error>> + Send;
}
