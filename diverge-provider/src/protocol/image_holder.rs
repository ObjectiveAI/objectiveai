//! Whether a provider holds an image for a caller, asked before
//! anything needs it.

use diverge_sdk::shared::Identity;
use std::future::Future;

use diverge_sdk::shared::containers::request::Image;

/// Answers one question: do you hold this image, for this caller.
///
/// [`images::has`](diverge_sdk::provider::endpoints::images::has) is
/// the whole of what it serves. A caller asks before committing to
/// anything — before building a request around an image, before
/// opening an [`images::oci`](diverge_sdk::provider::endpoints::images::oci)
/// scope to pull it, before waiting for a pull that was never going to
/// happen — and this is what a provider answers with.
///
/// # Why it is not a [`ContainerDeployer`] method
///
/// Because asking is not a small deploy. Nothing is created, nothing
/// is pulled, and the caller may never deploy anything at all — it may
/// be choosing between images, or carrying one to a run on another
/// provider, or asking on behalf of somebody else.
///
/// The two also fail differently in a way that matters.
/// [`deploy`](super::container_deployer::ContainerDeployer::deploy)
/// says "get me this", and not getting it is an error. This says "do
/// you", and no is an ANSWER — the distinction the endpoint is built
/// around, and one that would be lost inside a method whose only
/// failure channel is its error.
///
/// # Why it is told who is asking
///
/// Because the answer depends on it, and is meant to. A no does not
/// distinguish "I do not have it" from "I have it and will not serve
/// it to you", and that conflation is only worth anything if a
/// provider is allowed to consider the asker at all. A trait that did
/// not take an identity would make every image public or nothing.
///
/// It is the same identity a [`mount`](super::mount::Mount) carries and
/// a [`VolumeManager`](super::volume_manager::VolumeManager) takes:
/// whatever authenticated the connection, opaque here, never minted or
/// compared by this crate.
///
/// # Held is what an `images::oci` scope would serve
///
/// The answer is the same resolution the provider's
/// [`OciStore`](super::oci_store::OciStore) opens a scope with: the
/// caller's own Store — the images it built or pushed on this
/// provider through the
/// [`ImageManager`](super::image_manager::ImageManager), and no other
/// identity's — and then the image's references, each whose registry
/// the provider uses asked for the manifest under `<name>@<digest>`. A
/// yes here is an image the scope would open on; a no, one it would
/// answer `NotFound`. What the provider holds on its own account is
/// never asked: no image it pulled or cached for one caller is
/// revealed to another. A digest identifies the image and a reference
/// only states where its bytes may be fetched.
///
/// [`ContainerDeployer`]: super::container_deployer::ContainerDeployer
pub trait ImageHolder: Send + Sync {
    /// Whatever this provider's lookups fail with.
    ///
    /// Its own type, as with
    /// [`ContainerDeployer::Error`](super::container_deployer::ContainerDeployer::Error)
    /// and
    /// [`VolumeManager::Error`](super::volume_manager::VolumeManager::Error),
    /// and for the same reason: flattening one into the protocol's
    /// error is the handler's job, at the point where a frame is
    /// written.
    ///
    /// This is a failure to ANSWER, not a negative answer. A registry
    /// that could not be asked is one of these; an image a provider
    /// will not serve is `false` and [`Ok`].
    type Error: Send + 'static;

    /// Whether the image — its digest, and the references that state
    /// where its bytes may be fetched — is held for the caller.
    ///
    /// # A yes is not a promise
    ///
    /// It is what the provider believed when asked. An image can be
    /// deleted from under a caller between the question and the pull,
    /// and nothing here reserves anything or holds a lock — a caller
    /// that treats a yes as a guarantee has read more into it than was
    /// said.
    ///
    /// Which is the ordinary shape of the question. The alternative is
    /// an answer that pins an image for a caller that may never come
    /// back.
    fn has(
        &self,
        client_identity: &Identity,
        image: &Image,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send;
}
