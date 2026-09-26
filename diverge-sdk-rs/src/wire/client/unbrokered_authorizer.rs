//! Judging a provider's unbrokered credential, and saying who
//! presented it.

use std::future::Future;
use std::net::IpAddr;

/// How a caller decides whether a dialling provider may connect, and
/// who it is.
///
/// Consumed by [`authorize`](super::authorize::authorize) on an
/// [`Incoming`](super::authorization::Authorization::Incoming)
/// connection: the provider's first frame carries an
/// [`Auth::Unbrokered`](crate::wire::frame::auth::Auth::Unbrokered)
/// credential, and this is what judges it. What is IN the credential is
/// not this protocol's business — a bearer token, an API key, a signed
/// assertion — it means whatever the two ends agreed it means before
/// either of them dialled, and the party that made that agreement is
/// the party implementing this.
///
/// # The mirror, with the identity in it
///
/// The server half's
/// [`UnbrokeredAuthorizer`](crate::wire::server::unbrokered_authorizer::UnbrokeredAuthorizer),
/// with the direction inverted and nothing else: a provider that
/// dials a caller is judged the way a caller that dials a provider
/// is — a credential, an address, and an answer that names the peer.
/// What a caller does with a provider's name is the caller's own —
/// which provider an agent ran on, say, written where the agent's
/// record is kept — and a caller that wants none of it answers with
/// any string.
///
/// # The address is a signal
///
/// `address` is the peer the socket came from, as the OS reported it:
/// what a caller needs to refuse a source, to slow one down, or to
/// write down who presented a credential that did not pass. It is not
/// an identity and it does not become one — the identity is the
/// answer, and the answer comes from the credential.
///
/// # The answer is an identity
///
/// [`Ok`] is the provider's identity, as the caller will know the
/// provider from here on: the string
/// [`authorize`](super::authorize::authorize) hands back beside the
/// connection, and the same kind of opaque string the server half
/// produces for a caller. Where it comes from — a key the credential
/// equalled, a hook that answered — is the implementation's.
///
/// # A refusal earns the provider nothing
///
/// [`Err`] is a refusal, and it goes NOWHERE on the wire. The auth
/// frame's rule, in this direction as in the other: an accepted
/// credential is followed by the connection simply working, a
/// rejected one by a close, and a peer that has not authenticated
/// cannot make this end compose a reply. The error surfaces only in
/// what [`authorize`](super::authorize::authorize) returns, for the
/// caller's own log, and the connection it drops is the whole answer.
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

    /// Judge one credential, presented from `address`.
    ///
    /// [`Ok`] names the provider; [`Err`] refuses it, and the
    /// connection is over. There is no third answer, because the wire
    /// has no frame for one.
    fn authorize(
        &self,
        credential: &str,
        address: IpAddr,
    ) -> impl Future<Output = Result<String, Self::Error>> + Send;
}
