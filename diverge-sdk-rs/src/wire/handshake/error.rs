//! Why a handshake did not complete.

use std::fmt;

use crate::wire::frame::auth::AuthError;

/// What [`connect`](super::connect) and [`accept`](super::accept) fail
/// with. None of it reaches the other end, which sees only the close:
/// these are for the caller, which drops the connection, and its log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The connection ended before the other end's handshake frame
    /// arrived.
    Ended,
    /// The other end's frame was not a type `0` frame.
    NotAuth,
    /// The other end's handshake frame could not be read.
    Malformed(AuthError),
    /// The key the other end sent is not the one its token is bound
    /// to: its thumbprint is not the token's.
    Key,
    /// The other end's signature is not its key's over this
    /// handshake: it did not prove it holds the key.
    Signature,
    /// The key exchange yielded nothing: the other end's one-time key
    /// is no point of the curve.
    Exchange,
    /// This end has no token to present: why.
    Present(String),
    /// This end does not accept the other end's token: why.
    Refused(String),
    /// A token could not be sent: the connection is already over.
    Send(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Ended => f.write_str("the connection ended before the other end's handshake frame"),
            Error::NotAuth => f.write_str("the other end's frame was not a handshake frame"),
            Error::Malformed(error) => write!(f, "the other end's handshake frame could not be read: {error}"),
            Error::Key => f.write_str("the other end's key is not the one its token is bound to"),
            Error::Signature => f.write_str("the other end did not prove it holds the key its token names"),
            Error::Exchange => f.write_str("the key exchange yielded no shared secret"),
            Error::Present(why) => write!(f, "this end has no token to present: {why}"),
            Error::Refused(why) => write!(f, "the other end's token is not accepted: {why}"),
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
