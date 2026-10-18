//! The images a caller builds on the provider: the namespace, and the
//! verbs that make and unmake its names.

use std::future::Future;

use futures_util::Stream;

use super::context::Context;
use super::progress::Progress;
use diverge_sdk::provider::endpoints::images::delete::server::response::Deletion;
use diverge_sdk::provider::endpoints::images::list::server::response::Image;
use diverge_sdk::provider::endpoints::images::stat::server::response::Stat;

/// A namespace of named images, one per caller: the store of what a
/// caller built.
///
/// The four store endpoints of
/// [`images`](diverge_sdk::provider::endpoints::images) are four verbs
/// over one thing, and this is the thing: what a listing reports,
/// what a build adds and a delete takes away, and where a name is
/// looked up. Unlike a volume, an image is never acted on in place —
/// it is built once and then only listed, examined and destroyed —
/// so there is no second trait for the thing a name resolves to, and
/// everything is here.
///
/// # Every method takes a `client_identity`
///
/// Because an image's name is unique within the caller that built it,
/// not globally, and because the STORE is the caller's: what one
/// identity built is not listed to, examined by, run by or deleted by
/// another, however alike the names. The identity is an argument
/// rather than something the manager was built with, so that ONE
/// manager serves every caller on every connection. Where a provider
/// gets the identity is its own business — it is whatever
/// authenticated the connection, and this crate never mints one.
///
/// # An image built is an image held, for its builder
///
/// The [`ImageChecker`](super::image_checker::ImageChecker) and the
/// [`ContainerDeployer`](super::container_deployer::ContainerDeployer)
/// a provider supplies beside this treat an image an identity built
/// as one the provider holds for that identity — a check naming its
/// digest is available, a run naming it runs it from the store — and
/// as not there for any other identity, unless that identity built
/// the same image itself. How the three share the store is the
/// provider's; the rule is the protocol's.
///
/// # No eviction, and a capacity
///
/// Nothing a caller built is removed but by its
/// [`delete`](Self::delete). The store has a capacity, apart from
/// whatever the provider caches of images it pulls, and a
/// [`build`](Self::build) the store has no room for ends in
/// [`Progress::InsufficientCapacity`] — an answer, not an error —
/// with nothing built. What the capacity is, and whether it is one
/// identity's or every identity's, is the provider's.
///
/// # What is deliberately not decided
///
/// What earns a failure. The wire has one
/// [`Error`](diverge_sdk::shared::error::Error) per endpoint and says
/// nothing about when it is sent, so whether a build over a name the
/// caller already has fails, and what a provider accepts as a name,
/// are the provider's to answer. This trait gives it somewhere to say
/// no and does not say when. The cases the wire does decide each
/// have an answer of their own: no room is insufficient capacity, and
/// an image a running container runs is in use.
pub trait ImageManager: Send + Sync {
    /// Whatever this provider's images fail with.
    ///
    /// Its own type, not
    /// [`shared::error::Error`](diverge_sdk::shared::error::Error),
    /// for the reason
    /// [`VolumeManager::Error`](super::volume_manager::VolumeManager::Error)
    /// gives: flattening a provider's failure into the protocol's
    /// error is the handler's job, at the point where a frame is
    /// written. `Send + 'static` because it crosses tasks and
    /// outlives the call that produced it.
    type Error: Send + 'static;

    /// Which images this caller has built and not deleted.
    ///
    /// Everything the caller may name in this family, which is the
    /// whole of what a listing is for. Empty is a caller with no
    /// image, which is where every caller starts; it is not a
    /// failure. Nothing here sorts, and the endpoint does not
    /// either: a [`name`](Image::name) is the handle and a position
    /// is not.
    fn list(
        &self,
        client_identity: &str,
    ) -> impl Future<Output = Result<Vec<Image>, Self::Error>> + Send;

    /// The caller's image under `name`, as a listing reports it and
    /// with the Containerfile it was built from; or [`None`] where the
    /// caller has none by it.
    ///
    /// `None` is an answer: a name that is not in this caller's
    /// listing — never there, deleted, or another caller's — and the
    /// handler turns it into the endpoint's error itself. [`Err`] is
    /// the provider unable to look.
    fn stat(
        &self,
        client_identity: &str,
        name: &str,
    ) -> impl Future<Output = Result<Option<Stat>, Self::Error>> + Send;

    /// What [`build`](Self::build) hands back: the build as it goes,
    /// one [`Progress`] per item, ending at the image built or the
    /// word that there was no room — or at the provider's error, a
    /// build that failed, with nothing built. `'static`, since the
    /// handler polls it for the build's life.
    type Build: Stream<Item = Result<Progress, Self::Error>> + Send + 'static;

    /// Build an image for this caller, under `name`, from the
    /// `containerfile`, with `context` — a directory of one of the
    /// caller's volumes, resolved by the provider — as the build's
    /// context and working directory, or none.
    ///
    /// The name is the caller's to choose, and it is chosen HERE —
    /// this is the one place a name enters the namespace. What comes
    /// back is the build's own stream: every line the build tool says
    /// is a [`Progress::Output`], and the handler sends each to the
    /// caller as it comes, so a caller watches the build as a person
    /// would. The last item is the image built, which the manager has
    /// counted against the store, or that there was no room, or the
    /// error; a stream that ends with none of those is the handler's
    /// error to the caller, and nothing is taken to exist.
    ///
    /// # The context is held
    ///
    /// The handler has taken the volume's shared hold before calling
    /// this and gives it back after the stream ends, so the directory
    /// is as a mount would see it for the build's life, and a volume
    /// that could not be held was refused before this was asked. The
    /// manager reads the directory and writes nothing into it.
    fn build(
        &self,
        client_identity: &str,
        name: &str,
        containerfile: &str,
        context: Option<&Context>,
    ) -> impl Future<Output = Self::Build> + Send;

    /// Remove the caller's image under `name`, and give the room it
    /// took back to the store.
    ///
    /// [`Deletion::Deleted`] is the image gone, and the name free for
    /// a [`build`](Self::build) to use again; [`Deletion::InUse`] is a
    /// running container of the caller running the image, and nothing
    /// changed; [`None`] is a name the caller has no image by, which
    /// the handler turns into the endpoint's error. [`Err`] is the
    /// image as it was.
    fn delete(
        &self,
        client_identity: &str,
        name: &str,
    ) -> impl Future<Output = Result<Option<Deletion>, Self::Error>> + Send;
}
