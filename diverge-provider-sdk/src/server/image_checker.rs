//! Whether a provider would supply an image, asked before anything
//! needs it.

use std::future::Future;

use crate::endpoints::images::check::server::response::Response;

/// Answers one question: would you supply this image, to this caller.
///
/// [`images::check`](crate::endpoints::images::check) is the whole of
/// what it serves. A caller asks before committing to anything — before
/// building a request around an image, before waiting for a pull that
/// was never going to happen — and this is what a provider answers
/// with.
///
/// # Why it is not a [`ContainerDeployer`] method
///
/// Because a check is not a small deploy. Nothing is created, nothing
/// is pulled, and the caller may never deploy anything at all — it may
/// be choosing between images, or checking a list it holds, or asking
/// on behalf of somebody else.
///
/// The two also fail differently in a way that matters.
/// [`registry`](super::container_deployer::ContainerDeployer::registry)
/// says "get me this", and not getting it is an error. This says "would
/// you", and no is an ANSWER — the distinction the endpoint is built
/// around, and one that would be lost inside a method whose only
/// failure channel is its error.
///
/// A provider will usually implement both against the same store. That
/// is an implementation sharing a cache, not one question.
///
/// # Why it is told who is asking
///
/// Because the answer depends on it, and is meant to.
/// [`Unavailable`](crate::endpoints::images::check::server::response::Unavailable)
/// deliberately does not distinguish "I do not have it" from "I have it
/// and will not serve it to you" — and that conflation is only worth
/// anything if a provider is allowed to consider the asker at all. A
/// trait that did not take an identity would make every image public or
/// nothing.
///
/// It is the same identity a [`mount`](super::mount::Mount) carries and
/// a [`VolumeManager`](super::volume_manager::VolumeManager) takes:
/// whatever authenticated the connection, opaque here, never minted or
/// compared by this crate.
///
/// # There is no registry in the question
///
/// A digest identifies the image and a registry only locates one, so
/// where a provider would get it — its own mirror, a pull-through
/// cache, a private registry it holds credentials for, something it
/// already has on disk — is the provider's business and not part of
/// what is being asked. See
/// [`request::Frame`](crate::endpoints::images::check::client::request::Frame),
/// which says the same from the other side.
///
/// [`ContainerDeployer`]: super::container_deployer::ContainerDeployer
pub trait ImageChecker: Send + Sync {
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
    /// that timed out is one of these; an image a provider will not
    /// serve is an
    /// [`Unavailable`](crate::endpoints::images::check::server::response::Unavailable)
    /// and is [`Ok`].
    type Error: Send + 'static;

    /// Would you supply the image named by this repository and digest.
    ///
    /// [`Available`](crate::endpoints::images::check::server::response::Available)
    /// or
    /// [`Unavailable`](crate::endpoints::images::check::server::response::Unavailable),
    /// which is the endpoint's own answer type rather than a [`bool`].
    /// The endpoint already decided that two variants beat a boolean,
    /// on the grounds that only one of the two answers has anything
    /// more to say — so a `bool` here would have to become this the day
    /// an available image carries terms.
    ///
    /// # A yes is not a promise
    ///
    /// It is what the provider believed when asked. An image can be
    /// deleted from under a caller between the check and the deploy,
    /// and nothing here reserves anything or holds a lock — a caller
    /// that treats a yes as a guarantee has read more into it than was
    /// said.
    ///
    /// Which is the ordinary shape of the question. The alternative is
    /// a check that pins an image for a caller that may never come
    /// back.
    fn check(
        &self,
        client_identity: &str,
        name: &str,
        digest: &str,
    ) -> impl Future<Output = Result<Response, Self::Error>> + Send;
}
