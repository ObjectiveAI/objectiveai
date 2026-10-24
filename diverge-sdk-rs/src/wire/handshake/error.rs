//! Why a handshake did not complete.

use std::fmt;

use crate::wire::frame::auth::AuthError;

/// What [`connect`](super::connect) and [`accept`](super::accept) fail
/// with. None of it reaches the other end, which sees only the close:
/// these are for the caller, which drops the connection, and its log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The connection ended before the other end's token arrived.
    Ended,
    /// The other end's first frame was not a type `0` frame.
    NotAuth,
    /// The other end's token could not be read.
    Malformed(AuthError),
    /// This end has no token to present from the issuer: why.
    Present(String),
    /// This end does not accept the other end's token: why.
    Refused(String),
    /// The connectee answered from another issuer than the one the
    /// connector presented.
    Issuer {
        /// The issuer the connector presented.
        sent: String,
        /// The issuer the connectee answered from.
        answered: String,
    },
    /// A token could not be sent: the connection is already over.
    Send(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Ended => f.write_str("the connection ended before the other end's token"),
            Error::NotAuth => f.write_str("the other end's first frame was not its token"),
            Error::Malformed(error) => write!(f, "the other end's token could not be read: {error}"),
            Error::Present(why) => write!(f, "this end has no token to present: {why}"),
            Error::Refused(why) => write!(f, "the other end's token is not accepted: {why}"),
            Error::Issuer { sent, answered } => {
                write!(f, "the connectee answered from `{answered}`, not `{sent}`, the issuer presented")
            }
            Error::Send(why) => write!(f, "the token could not be sent: {why}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Malformed(error) => Some(error),
            _ => None,
        }
    }
}
