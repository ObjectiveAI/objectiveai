//! Who is on the other end, and on whose word.

use std::fmt;
use std::str::{self, Utf8Error};

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// The payload of an [`Auth`](super::client::ClientFrame::Auth) frame,
/// in either direction: the discover flag, and a token.
///
/// One type for both ends, because each end sends exactly one — the
/// connector first, the connectee in answer — and the frame is the same
/// whichever way it travels. See [`handshake`](crate::wire::handshake)
/// for the rules of the exchange.
///
/// # The layout
///
/// 1. **The flag.** The first byte: `0`, the sender is not
///    discoverable, or `1`, it is. Any other byte is malformed. Only a
///    provider honors it — it lists the sender to its
///    [`discover`](crate::provider::endpoints::discover) — and every
///    other receiver reads it and ignores it.
/// 2. **The token.** Every byte after the first: an RFC 7519 JWT as
///    UTF-8, and nothing else — no length, since WebSocket delimits the
///    message. Its `iss` names the issuer it is from, which is how the
///    connectee knows which issuer to answer from. An empty token is
///    malformed.
///
/// What else is in the token is not this layer's business: it is not
/// parsed here, not checked beyond being UTF-8 and not empty, and not
/// bounded in length. Each end judges the other's by its own means.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Auth<'a> {
    /// Whether the sender may be listed by a provider's `discover`.
    pub discover: bool,
    /// The token, the payload's every byte after the flag.
    pub token: &'a str,
}

/// The flag's byte, then the token's own bytes.
impl Encode for Auth<'_> {
    /// [`Infallible`](std::convert::Infallible): a byte and a string's
    /// own bytes.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[u8::from(self.discover)]);
        out.extend_from_slice(self.token.as_bytes());
        Ok(())
    }
}

impl<'a> Decode<'a> for Auth<'a> {
    /// Empty, a flag that is neither `0` nor `1`, an empty token, or a
    /// token that is not UTF-8.
    type Error = AuthError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        let Some((&flag, token)) = bytes.split_first() else {
            return Err(AuthError::Empty);
        };
        let discover = match flag {
            0 => false,
            1 => true,
            other => return Err(AuthError::Discover(other)),
        };
        if token.is_empty() {
            return Err(AuthError::Empty);
        }
        let token = str::from_utf8(token).map_err(AuthError::Token)?;
        Ok(Auth { discover, token })
    }
}

/// A payload that could not be read. Its receiver closes the
/// connection, and sends nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthError {
    /// No bytes at all, or a flag and no token.
    Empty,
    /// The flag was neither `0` nor `1`.
    Discover(u8),
    /// The token was not UTF-8.
    Token(Utf8Error),
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::Empty => f.write_str("the auth payload holds no token"),
            AuthError::Discover(byte) => write!(f, "the discover flag is {byte}, neither 0 nor 1"),
            AuthError::Token(error) => write!(f, "the token is not utf-8: {error}"),
        }
    }
}

impl std::error::Error for AuthError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AuthError::Token(error) => Some(error),
            AuthError::Empty | AuthError::Discover(_) => None,
        }
    }
}
