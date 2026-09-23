//! The handshake, before the connection is anything else.

use std::fmt;
use std::net::IpAddr;

use bytes::Bytes;
use futures_util::{SinkExt as _, StreamExt as _};

use super::authorization::{self, Authorization};
use super::authorized::Authorized;
use super::unbrokered_authorizer::UnbrokeredAuthorizer;
use crate::connection::Connection;
use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::frame::auth;
use crate::frame::client::ClientFrame;
use crate::frame::server::ServerFrame;

/// Authenticate a connection, in whichever direction it needs.
///
/// The first step of using one, before the split and before anything
/// is built: the credential is the connection's first frame and
/// nothing may precede it, so this takes the [`Connection`] whole and
/// hands it back once the handshake is done, as an [`Authorized`] —
/// the connection [`Router::new`](super::router::Router::new) and
/// [`Handle::new`](super::handle::Handle::new) are built from, and the
/// provider's identity beside it.
///
/// The mirror of the handshake at the top of the server's
/// [`handle`](crate::server::handle::handle), with the dial direction
/// inverted: the identity comes from the same two places, the
/// argument on one variant and the [`UnbrokeredAuthorizer`] on the
/// other, and `address` is the peer's, as the OS reported it, for the
/// authorizer to see.
///
/// # [`Outgoing`](Authorization::Outgoing) sends and is done
///
/// This end dialled, so this end authenticates: the credential goes
/// out and the connection comes back, with nothing read and nothing
/// waited for, under the identity the caller supplied. There is no
/// answer to wait FOR — an accepted credential is followed by the
/// connection simply working, a rejected one by a close, and the
/// caller meets either through its ordinary reads. A send that does
/// not land is ignored for the same reason: the connection is already
/// over, and the very next read will say so.
///
/// # [`Incoming`](Authorization::Incoming) demands the credential
/// first
///
/// The provider dialled, so the provider speaks first, and the first
/// thing it says must be who it is. The first well-formed frame that
/// is not a credential, a credential that will not decode, and a
/// credential the authorizer refuses are all [`Err`] — and all of them
/// send the provider NOTHING. A peer that has not authenticated cannot
/// make this end compose a reply; the caller drops the connection, and
/// the close is the whole answer.
///
/// Frames that will not decode at all are skipped rather than judged,
/// the same way every read loop in this crate skips them — the first
/// frame this end can READ is the one that must be the credential. A
/// connection that ends before producing one is
/// [`Ended`](AuthorizeError::Ended): nothing was judged and nothing
/// was served, and there is no identity to serve it under, so there
/// is nothing to hand back.
///
/// # After this, a credential is never legitimate again
///
/// A connection has one, presented by the side that dialled, before
/// everything else. The rule's other half lives where the rest of the
/// connection is read: [`Router::run`](super::router::Router::run)
/// ends with an error on any credential it sees, this function having
/// already consumed the only lawful one.
pub async fn authorize<U>(
    mut connection: Connection,
    authorization: Authorization<U>,
    address: IpAddr,
) -> Result<Authorized, AuthorizeError<U::Error>>
where
    U: UnbrokeredAuthorizer,
{
    match authorization {
        Authorization::Outgoing {
            auth,
            provider_identity,
        } => {
            let mut payload = Vec::new();
            match &auth {
                authorization::Auth::Unbrokered(credential) => {
                    auth::Auth::Unbrokered(credential)
                        .encode(&mut Writer::new(&mut payload))
                        .unwrap_or_else(|error| match error {});
                }
            }
            let mut buffer = Vec::new();
            ClientFrame::Auth { payload: &payload }
                .encode(&mut Writer::new(&mut buffer))
                .unwrap_or_else(|error| match error {});
            let _ = connection.send(Bytes::from(buffer)).await;
            Ok(Authorized {
                connection,
                provider_identity,
            })
        }
        Authorization::Incoming { unbrokered } => loop {
            let Some(received) = connection.next().await else {
                // Gone before speaking. Nothing was judged, so there
                // is nobody to serve the connection as.
                return Err(AuthorizeError::Ended);
            };
            // A transport error yields no frame, and a frame this end
            // cannot read is not one to judge. The loop takes the next,
            // which is every read loop's rule restated here.
            let Ok(bytes) = received else { continue };
            let Ok(frame) = ServerFrame::decode(&bytes) else { continue };
            let ServerFrame::Auth { payload } = frame else {
                // The provider skipped the handshake. Whatever this
                // frame was, it is dropped unanswered: a peer that has
                // not authenticated cannot make this end compose a
                // reply.
                return Err(AuthorizeError::Unauthenticated);
            };
            let credential = match auth::Auth::decode(payload) {
                Ok(auth::Auth::Unbrokered(credential)) => credential,
                Err(error) => return Err(AuthorizeError::Auth(error)),
            };
            return match unbrokered.authorize(credential, address).await {
                Ok(provider_identity) => Ok(Authorized {
                    connection,
                    provider_identity,
                }),
                Err(error) => Err(AuthorizeError::Unauthorized(error)),
            };
        },
    }
}

/// A connection that did not make it past the handshake.
///
/// Only an [`Incoming`](Authorization::Incoming) connection can fail
/// this way — [`Outgoing`](Authorization::Outgoing) sends and is done,
/// its acceptance or rejection arriving as a connection that works or
/// one that closed.
///
/// # None of this reaches the provider
///
/// The auth frame's rule, in this direction as in the other: a peer
/// that has not authenticated earns no bytes, and these variants are
/// for the CALLER, which is the party that drops the connection when
/// this returns and may want its log to say why.
#[derive(Debug)]
pub enum AuthorizeError<E> {
    /// The connection ended before the provider presented anything.
    ///
    /// Not a refusal and not a defect — the ordinary ending, arrived
    /// early — but an error all the same, because a connection that
    /// nobody was judged on has no identity to be served under, and
    /// nothing else this returns is built without one.
    Ended,
    /// The provider's first frame was not its credential.
    ///
    /// Whatever it was, the provider skipped the handshake, and
    /// nothing it sends is worth reading.
    Unauthenticated,
    /// The credential would not decode.
    ///
    /// A mode this version does not know, or a credential that was not
    /// UTF-8. Not a refusal — nothing was ever judged — but the same
    /// close, because a credential this end cannot read authenticates
    /// nobody.
    Auth(auth::AuthError),
    /// The authorizer refused the credential, and this is its reason.
    ///
    /// The one variant that is a decision rather than a defect, in the
    /// caller's own vocabulary — see
    /// [`UnbrokeredAuthorizer::Error`].
    Unauthorized(E),
}

impl<E> fmt::Display for AuthorizeError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthorizeError::Ended => f.write_str(
                "the connection ended before the provider presented a credential",
            ),
            AuthorizeError::Unauthenticated => f.write_str(
                "the provider's first frame was not its credential",
            ),
            AuthorizeError::Auth(error) => {
                write!(f, "the credential could not be read: {error}")
            }
            AuthorizeError::Unauthorized(_) => {
                f.write_str("the authorizer refused the credential")
            }
        }
    }
}

/// [`Unauthorized`](AuthorizeError::Unauthorized) is not the source,
/// even though it wraps the reason: `E` is unbounded, like every error
/// a caller supplies, and a bound here would be charged to every
/// implementation for a `source` chain nobody reads.
impl<E> std::error::Error for AuthorizeError<E>
where
    E: fmt::Debug,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AuthorizeError::Auth(error) => Some(error),
            AuthorizeError::Ended
            | AuthorizeError::Unauthenticated
            | AuthorizeError::Unauthorized(_) => None,
        }
    }
}
