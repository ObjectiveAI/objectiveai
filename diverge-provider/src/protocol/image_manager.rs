//! The Store: the images a caller owns on the provider, by digest, and
//! the verbs that gain, describe and give up an ownership.

use diverge_sdk::shared::Identity;
use std::future::Future;

use futures_util::Stream;

use super::context::Context;
use super::image_source::ImageSource;
use super::progress::{Progress, Pushing};
use diverge_sdk::provider::endpoints::images::delete::server::response::Deletion;
use diverge_sdk::provider::endpoints::images::list::server::response::Image;
use diverge_sdk::provider::endpoints::images::stat::server::response::Stat;
use diverge_sdk::shared::containers::request::Image as Wanted;
use diverge_sdk::shared::description::Description;

/// The Store: one per caller, of the images it owns, each by its
/// manifest digest alone.
///
/// The store endpoints of
/// [`images`](diverge_sdk::provider::endpoints::images) are verbs over
/// one thing, and this is the thing: what a listing reports, what a
/// build or a push adds and a delete takes away, what an edit
/// describes. An image is never acted on in place, so there is no
/// second trait for the thing a digest resolves to.
///
/// # Every method takes a `client_identity`
///
/// Because the STORE is the caller's: what one identity owns is not
/// listed to, examined, described, run from or deleted by another.
/// Two identities owning one digest each own it. The identity
/// is an argument so that ONE manager serves every caller on every
/// connection.
///
/// # Ownership is proven when it is gained
///
/// A build proves it by building. A push from a registry the provider
/// uses takes the registry's bytes, verified against the digest; a
/// push from the caller ends with the nonce check of
/// [`prove`](diverge_sdk::shared::containers::oci::prove) over the
/// provider's own copy, whether or not a byte moved, and only a pass
/// is an ownership. A digest the caller owns already is never pushed:
/// it is [`Pushing::Exists`].
///
/// # An image owned is an image held, for its owner
///
/// The [`OciStore`](super::oci_store::OciStore) and the
/// [`ContainerDeployer`](super::container_deployer::ContainerDeployer)
/// a provider supplies beside this treat an image an identity owns as
/// one the provider holds for that identity — a pull naming its
/// digest is held, served and proven, a run naming it runs it from
/// the Store with no nonce check — and as not there for any other
/// identity, unless that identity owns the same digest itself.
///
/// # No eviction, and a capacity
///
/// Nothing a caller owns is removed but by its
/// [`delete`](Self::delete). The Store has a capacity, and a
/// [`build`](Self::build) or a [`push`](Self::push) it has no room for
/// ends in insufficient capacity — an answer, not an error — with
/// nothing owned. What the capacity is, and how the provider keeps the
/// bytes, are the provider's.
pub trait ImageManager: Send + Sync {
    /// Whatever this provider's Store fails with: flattened into the
    /// protocol's error by the handler, at the point where a frame is
    /// written. `Send + 'static` because it crosses tasks.
    type Error: Send + 'static;

    /// Which images this caller owns. Empty is a caller with no
    /// image, which is where every caller starts. Nothing here sorts:
    /// a [`digest`](Image::digest) is the handle.
    fn list(
        &self,
        client_identity: &Identity,
    ) -> impl Future<Output = Result<Vec<Image>, Self::Error>> + Send;

    /// The caller's image under `digest`, as a listing reports it,
    /// with the Containerfile it was built from when it was built
    /// here; [`None`] where the caller owns no such digest, which the
    /// handler answers itself.
    fn stat(
        &self,
        client_identity: &Identity,
        digest: &str,
    ) -> impl Future<Output = Result<Option<Stat>, Self::Error>> + Send;

    /// What [`build`](Self::build) hands back: the build as it goes,
    /// ending at the image owned, the digest owned already, no room,
    /// or the provider's error. `'static`, since the handler polls it
    /// for the build's life.
    type Build: Stream<Item = Result<Progress, Self::Error>> + Send + 'static;

    /// Build an image for this caller from the `containerfile`, with
    /// `context` — a directory of one of the caller's volumes,
    /// resolved by the provider — as the build's context, or none; the
    /// image owned under the digest the build produced, with
    /// `description`. A digest the caller owns already is
    /// [`Progress::Exists`], and nothing changes.
    ///
    /// # The context is held
    ///
    /// The handler has taken the volume's shared hold before calling
    /// this and gives it back after the stream ends.
    fn build(
        &self,
        client_identity: &Identity,
        description: Option<&Description>,
        containerfile: &str,
        context: Option<&Context>,
    ) -> impl Future<Output = Self::Build> + Send;

    /// What [`push`](Self::push) hands back: every blob taken from the
    /// caller, then exactly one end — see [`Pushing`] — or the
    /// provider's error. `'static`, since the handler polls it for the
    /// push's life.
    type Push: Stream<Item = Result<Pushing, Self::Error>> + Send + 'static;

    /// Take the image `image` names into this caller's Store, with
    /// `description`: from a registry the provider uses when one
    /// serves it, else from the caller through `source`, ending with
    /// the nonce check. The order and the answers are the endpoint's —
    /// see [`push`](diverge_sdk::provider::endpoints::images::push).
    fn push(
        &self,
        client_identity: &Identity,
        image: &Wanted,
        description: Option<&Description>,
        source: ImageSource,
    ) -> impl Future<Output = Self::Push> + Send;

    /// The description of the caller's image under `digest` set, or
    /// cleared with `None`. [`None`] is a digest the caller owns no
    /// image by; `Some(changed)` is the edit made, `changed` whether
    /// the description is other than it was — the handler tells the
    /// listings only then.
    fn edit(
        &self,
        client_identity: &Identity,
        digest: &str,
        description: Option<&Description>,
    ) -> impl Future<Output = Result<Option<bool>, Self::Error>> + Send;

    /// Give up the caller's ownership of the image under `digest`.
    ///
    /// [`Deletion::Deleted`] is the ownership gone; [`Deletion::InUse`]
    /// is a running container running the image, and nothing changed;
    /// [`None`] is a digest the caller owns no image by. [`Err`] is the
    /// Store as it was.
    fn delete(
        &self,
        client_identity: &Identity,
        digest: &str,
    ) -> impl Future<Output = Result<Option<Deletion>, Self::Error>> + Send;
}
