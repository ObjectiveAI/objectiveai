//! Who is on the other end, and on whose word.

use std::fmt;
use std::str::{self, Utf8Error};

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// The payload of an [`Auth`](super::client::ClientFrame::Auth) frame,
/// in either direction: a token, and nothing else.
///
/// One type for both ends, because each end sends exactly one — the
/// connector first, the connectee in answer — and the frame is the same
/// whichever way it travels. See [`handshake`](crate::wire::handshake)
/// for the rules of the exchange.
///
/// # The layout
///
/// 1. **The token.** Every byte of the payload: an RFC 7519 JWT as
///    UTF-8, and nothing else — no flag before it and no length, since
///    WebSocket delimits the message.
/// 2. **Malformed.** An empty payload, and a payload that is not
///    UTF-8.
///
/// What else is in the token is not this layer's business: it is not
/// parsed here, not checked beyond being UTF-8 and not empty, and not
/// bounded in length. Each end judges the other's by its own means.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Auth<'a> {
    /// The token, the payload's every byte.
    pub token: &'a str,
}

/// The token's own bytes.
impl Encode for Auth<'_> {
    /// [`Infallible`](std::convert::Infallible): a token is its bytes.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(self.token.as_bytes());
        Ok(())
    }
}

impl<'a> Decode<'a> for Auth<'a> {
    /// Empty, or not UTF-8.
    type Error = AuthError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        if bytes.is_empty() {
            return Err(AuthError::Empty);
        }
        let token = str::from_utf8(bytes).map_err(AuthError::Utf8)?;
        Ok(Auth { token })
    }
}

/// A payload that could not be read. Its receiver closes the
/// connection, and sends nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    /// No bytes at all.
    Empty,
    /// The token was not UTF-8.
    Utf8(Utf8Error),
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::Empty => f.write_str("the auth payload holds no token"),
            AuthError::Utf8(error) => write!(f, "the auth payload is not utf-8: {error}"),
        }
    }
}

impl std::error::Error for AuthError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AuthError::Utf8(error) => Some(error),
            AuthError::Empty => None,
        }
    }
}
