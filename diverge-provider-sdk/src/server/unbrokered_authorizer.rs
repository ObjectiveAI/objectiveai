//! Judging an unbrokered credential, and saying who presented it.

use std::future::Future;

/// How a provider decides whether a dialling peer may connect, and who
/// it is.
///
/// Consumed by [`handle`](super::handle::handle) on an
/// [`Incoming`](super::authorization::Authorization::Incoming)
/// connection: the peer's first frame carries an
/// [`Auth::Unbrokered`](crate::frame::auth::Auth::Unbrokered)
/// credential, and this is what judges it. What is IN the credential is
/// not this protocol's business — a bearer token, an API key, a signed
/// assertion — it means whatever the two ends agreed it means before
/// either of them dialled, and the party that made that agreement is
/// the party implementing this.
///
/// # The answer is an identity
///
/// [`Ok`] is the `client_identity` that everything downstream receives:
/// every handler, and through them the
/// [`ContainerDeployer`](super::container_deployer::ContainerDeployer),
/// the [`VolumeManager`](super::volume_manager::VolumeManager), the
/// [`ImageChecker`](super::image_checker::ImageChecker) and every
/// [`Mount`](super::mount::Mount). It is the same opaque string those
/// have taken all along; this is where it finally comes from.
///
/// # A refusal earns the peer nothing
///
/// [`Err`] is a refusal, and it goes NOWHERE on the wire. The auth
/// frame has documented this from the start: an accepted credential is
/// followed by the connection simply working, a rejected one by a
/// close, and a peer that has not authenticated cannot make this end
/// compose a reply — no bytes to amplify, no answer to read a reason
/// out of. The error surfaces only in what
/// [`handle`](super::handle::handle) returns, for the provider's own
/// log.
///
/// Which is also why the error type is the provider's own, like every
/// error on this half: nothing here branches on it, and this crate
/// names nothing it cannot see.
///
/// # `Send + Sync`, and async
///
/// Shared like the other capabilities, because a provider serves many
/// connections at once. Async because judging a credential is a lookup
/// — a database, an upstream, a cache — and not a string comparison,
/// even when today's implementation happens to be one.
pub trait UnbrokeredAuthorizer: Send + Sync {
    /// Why a credential was refused.
    ///
    /// [`Send`] and `'static` for the same reasons every other error
    /// here is: the future carrying it is [`Send`], so its output has
    /// to be, and it outlives the call that produced it.
    type Error: Send + 'static;

    /// Judge one credential.
    ///
    /// [`Ok`] names the peer; [`Err`] refuses it, and the connection is
    /// over. There is no third answer, because the wire has no frame
    /// for one.
    fn authorize(
        &self,
        credential: &str,
    ) -> impl Future<Output = Result<String, Self::Error>> + Send;
}
