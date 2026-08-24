//! Judging a provider's unbrokered credential.

use std::future::Future;

/// How a caller decides whether a dialling provider may connect.
///
/// Consumed by [`authorize`](super::authorize::authorize) on an
/// [`Incoming`](super::authorization::Authorization::Incoming)
/// connection: the provider's first frame carries an
/// [`Auth::Unbrokered`](crate::frame::auth::Auth::Unbrokered)
/// credential, and this is what judges it. What is IN the credential is
/// not this protocol's business — it means whatever the two ends agreed
/// it means before either of them dialled, and the party that made that
/// agreement is the party implementing this.
///
/// # The mirror with no identity in it
///
/// The server half's
/// [`UnbrokeredAuthorizer`](crate::server::unbrokered_authorizer::UnbrokeredAuthorizer)
/// answers with WHO the peer is, because everything downstream of a
/// provider — deployers, volumes, mounts — acts on a caller's behalf
/// and has to know whose. Nothing on this half asks. A caller talks to
/// the provider it chose or accepted, its executors carry no provider
/// identity, and what it needed to know about a provider that dialled
/// it was settled by whatever arrangement led to the dial. So [`Ok`]
/// here is `()`: the credential was acceptable, and that is the whole
/// of what acceptance means.
///
/// # A refusal earns the provider nothing
///
/// The auth frame's rule, in this direction as in the other: an
/// accepted credential is followed by the connection simply working, a
/// rejected one by a close, and a peer that has not authenticated
/// cannot make this end compose a reply. [`Err`] surfaces only in what
/// [`authorize`](super::authorize::authorize) returns, for the caller's
/// own log, and the connection it drops is the whole answer.
///
/// # `Send + Sync`, and async
///
/// Shared because a caller may be dialled by more than one provider at
/// once; async because judging a credential is a lookup, not a string
/// comparison, even when today's implementation happens to be one.
pub trait UnbrokeredAuthorizer: Send + Sync {
    /// Why a credential was refused.
    ///
    /// The caller's own, like every error a caller supplies. [`Send`]
    /// and `'static` for the same reasons every other one is: the
    /// future carrying it is [`Send`], so its output has to be, and it
    /// outlives the call that produced it.
    type Error: Send + 'static;

    /// Judge one credential.
    ///
    /// [`Ok`] accepts it; [`Err`] refuses it, and the connection is
    /// over. There is no third answer, because the wire has no frame
    /// for one.
    fn authorize(
        &self,
        credential: &str,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
}
