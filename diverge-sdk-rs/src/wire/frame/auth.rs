//! Who is on the other end, and on whose word.

use std::fmt;
use std::str::{self, Utf8Error};

use crate::shared::discover::{Description, DescriptionError, Discover};
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// The payload of an [`Auth`](super::client::ClientFrame::Auth) frame,
/// in either direction: the discover flag, a description with it, and a
/// token.
///
/// One type for both ends, because each end sends exactly one — the
/// connector first, the connectee in answer — and the frame is the same
/// whichever way it travels. See [`handshake`](crate::wire::handshake)
/// for the rules of the exchange.
///
/// # The layout
///
/// 1. **The flag.** The first byte: `0`, the sender is not
///    discoverable; `1`, it is, with no description; or `2`, it is,
///    with a description. Any other byte is malformed. Only a provider
///    honors it — it lists the sender to its
///    [`discover`](crate::provider::endpoints::discover) — and every
///    other receiver reads it and ignores it.
/// 2. **The description.** After the flag `2` only: its length in
///    bytes as a `u16`, big-endian, and then that many bytes, the
///    [`Description`] as UTF-8. A length past the payload, bytes that
///    are not UTF-8, and text that breaks a rule of `Description` are
///    malformed.
/// 3. **The token.** Every byte after those: an RFC 7519 JWT as UTF-8,
///    and nothing else — no length, since WebSocket delimits the
///    message. Its `iss` names the issuer it is from, which is how the
///    connectee knows which issuer to answer from. An empty token is
///    malformed.
///
/// What else is in the token is not this layer's business: it is not
/// parsed here, not checked beyond being UTF-8 and not empty, and not
/// bounded in length. Each end judges the other's by its own means.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Auth<'a> {
    /// Whether the sender may be listed by a provider's `discover`,
    /// and with what description.
    pub discover: Discover,
    /// The token, the payload's every byte after the flag and the
    /// description.
    pub token: &'a str,
}

/// Tag for [`Discover::Hidden`].
const HIDDEN: u8 = 0;

/// Tag for [`Discover::Listed`] with no description.
const LISTED: u8 = 1;

/// Tag for [`Discover::Listed`] with a description.
const DESCRIBED: u8 = 2;

/// The flag's byte, the description's length and bytes after the flag
/// `2`, then the token's own bytes.
impl Encode for Auth<'_> {
    /// [`Infallible`](std::convert::Infallible): a description is at
    /// most 132 characters, so its length always fits a `u16`.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match &self.discover {
            Discover::Hidden => out.extend_from_slice(&[HIDDEN]),
            Discover::Listed(None) => out.extend_from_slice(&[LISTED]),
            Discover::Listed(Some(description)) => {
                let text = description.as_str().as_bytes();
                out.extend_from_slice(&[DESCRIBED]);
                out.extend_from_slice(&(text.len() as u16).to_be_bytes());
                out.extend_from_slice(text);
            }
        }
        out.extend_from_slice(self.token.as_bytes());
        Ok(())
    }
}

impl<'a> Decode<'a> for Auth<'a> {
    /// Empty, a flag that is none of `0`, `1` and `2`, a description
    /// cut short or not one, an empty token, or a token that is not
    /// UTF-8.
    type Error = AuthError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        let Some((&flag, rest)) = bytes.split_first() else {
            return Err(AuthError::Empty);
        };
        let (discover, token) = match flag {
            HIDDEN => (Discover::Hidden, rest),
            LISTED => (Discover::Listed(None), rest),
            DESCRIBED => {
                let Some((length, rest)) = rest.split_first_chunk::<2>() else {
                    return Err(AuthError::Short);
                };
                let length = usize::from(u16::from_be_bytes(*length));
                if rest.len() < length {
                    return Err(AuthError::Short);
                }
                let (text, rest) = rest.split_at(length);
                let text = str::from_utf8(text).map_err(AuthError::Utf8)?;
                let description = Description::new(text).map_err(AuthError::Description)?;
                (Discover::Listed(Some(description)), rest)
            }
            other => return Err(AuthError::Discover(other)),
        };
        if token.is_empty() {
            return Err(AuthError::Empty);
        }
        let token = str::from_utf8(token).map_err(AuthError::Utf8)?;
        Ok(Auth { discover, token })
    }
}

/// A payload that could not be read. Its receiver closes the
/// connection, and sends nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    /// No bytes at all, or a flag and no token.
    Empty,
    /// The flag was none of `0`, `1` and `2`.
    Discover(u8),
    /// The description's length, or its bytes, ran past the payload.
    Short,
    /// The description, or the token, was not UTF-8.
    Utf8(Utf8Error),
    /// The description broke a rule.
    Description(DescriptionError),
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::Empty => f.write_str("the auth payload holds no token"),
            AuthError::Discover(byte) => write!(f, "the discover flag is {byte}, none of 0, 1 and 2"),
            AuthError::Short => f.write_str("the auth payload's description runs past its end"),
            AuthError::Utf8(error) => write!(f, "the auth payload is not utf-8: {error}"),
            AuthError::Description(error) => write!(f, "the auth payload's description is not one: {error}"),
        }
    }
}

impl std::error::Error for AuthError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AuthError::Utf8(error) => Some(error),
            AuthError::Description(error) => Some(error),
            AuthError::Empty | AuthError::Discover(_) | AuthError::Short => None,
        }
    }
}
