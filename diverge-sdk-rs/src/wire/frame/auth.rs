//! Who is on the other end, and by what proof.

use std::fmt;
use std::str::{self, Utf8Error};

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// The length of a public key, in bytes: an X25519 one-time key.
pub const KEY_LEN: usize = 32;

/// The length of a signature, in bytes: an Ed25519 one.
pub const SIGNATURE_LEN: usize = 64;

/// The payload of the connector's first
/// [`Auth`](super::client::ClientFrame::Auth) frame, the first frame of
/// a connection: its one-time public key, then its token. See
/// [`handshake`](crate::wire::handshake) for the rules of the exchange.
///
/// # The layout
///
/// 1. **The one-time key.** The first [`KEY_LEN`] bytes: an X25519
///    public key, RFC 7748, made for this connection and no other.
/// 2. **The token.** Every byte after it: an RFC 7519 JWT as UTF-8,
///    and nothing else — no length, since WebSocket delimits the
///    message.
/// 3. **Malformed.** A payload shorter than the key, a payload with no
///    byte after the key, and a token that is not UTF-8.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer<'a> {
    /// The connector's one-time public key.
    pub key: [u8; KEY_LEN],
    /// The connector's token.
    pub token: &'a str,
}

/// The payload of the connectee's one
/// [`Auth`](super::client::ClientFrame::Auth) frame, in answer to an
/// [`Offer`] it accepts: its one-time public key, its signature, then
/// its token.
///
/// # The layout
///
/// 1. **The one-time key.** The first [`KEY_LEN`] bytes: an X25519
///    public key, made for this connection and no other.
/// 2. **The signature.** The next [`SIGNATURE_LEN`] bytes: an Ed25519
///    signature, RFC 8032, by the key the connectee's token names, over
///    the handshake's transcript as the connectee signs it.
/// 3. **The token.** Every byte after it: an RFC 7519 JWT as UTF-8.
/// 4. **Malformed.** A payload shorter than the key and the signature,
///    a payload with no byte after them, and a token that is not
///    UTF-8.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer<'a> {
    /// The connectee's one-time public key.
    pub key: [u8; KEY_LEN],
    /// The connectee's signature over the transcript.
    pub signature: [u8; SIGNATURE_LEN],
    /// The connectee's token.
    pub token: &'a str,
}

/// The payload of the connector's second
/// [`Auth`](super::client::ClientFrame::Auth) frame, the handshake's
/// last: its signature, and nothing else.
///
/// # The layout
///
/// 1. **The signature.** Exactly [`SIGNATURE_LEN`] bytes: an Ed25519
///    signature by the key the connector's token names, over the
///    handshake's transcript as the connector signs it.
/// 2. **Malformed.** A payload of any other length.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proof {
    /// The connector's signature over the transcript.
    pub signature: [u8; SIGNATURE_LEN],
}

/// The key, then the token's own bytes.
impl Encode for Offer<'_> {
    /// [`Infallible`](std::convert::Infallible): the fields are their
    /// bytes.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&self.key);
        out.extend_from_slice(self.token.as_bytes());
        Ok(())
    }
}

impl<'a> Decode<'a> for Offer<'a> {
    /// Short, no token, or not UTF-8.
    type Error = AuthError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        let (key, rest) = bytes.split_first_chunk::<KEY_LEN>().ok_or(AuthError::Short)?;
        Ok(Offer { key: *key, token: token(rest)? })
    }
}

/// The key, the signature, then the token's own bytes.
impl Encode for Answer<'_> {
    /// [`Infallible`](std::convert::Infallible): the fields are their
    /// bytes.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&self.key);
        out.extend_from_slice(&self.signature);
        out.extend_from_slice(self.token.as_bytes());
        Ok(())
    }
}

impl<'a> Decode<'a> for Answer<'a> {
    /// Short, no token, or not UTF-8.
    type Error = AuthError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        let (key, rest) = bytes.split_first_chunk::<KEY_LEN>().ok_or(AuthError::Short)?;
        let (signature, rest) = rest.split_first_chunk::<SIGNATURE_LEN>().ok_or(AuthError::Short)?;
        Ok(Answer { key: *key, signature: *signature, token: token(rest)? })
    }
}

/// The signature's own bytes.
impl Encode for Proof {
    /// [`Infallible`](std::convert::Infallible): a signature is its
    /// bytes.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&self.signature);
        Ok(())
    }
}

impl Decode<'_> for Proof {
    /// Short, or long.
    type Error = AuthError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (signature, rest) = bytes.split_first_chunk::<SIGNATURE_LEN>().ok_or(AuthError::Short)?;
        if !rest.is_empty() {
            return Err(AuthError::Long);
        }
        Ok(Proof { signature: *signature })
    }
}

/// The token `bytes` are: every one of them, as UTF-8, and at least
/// one.
fn token(bytes: &[u8]) -> Result<&str, AuthError> {
    if bytes.is_empty() {
        return Err(AuthError::NoToken);
    }
    str::from_utf8(bytes).map_err(AuthError::Utf8)
}

/// A payload that could not be read. Its receiver closes the
/// connection, and sends nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    /// Fewer bytes than the fixed fields the payload begins with.
    Short,
    /// No byte after the fixed fields, where a token follows them.
    NoToken,
    /// The token was not UTF-8.
    Utf8(Utf8Error),
    /// A byte after the signature, where nothing follows it.
    Long,
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::Short => f.write_str("the auth payload is shorter than its fixed fields"),
            AuthError::NoToken => f.write_str("the auth payload holds no token"),
            AuthError::Utf8(error) => write!(f, "the auth payload's token is not utf-8: {error}"),
            AuthError::Long => f.write_str("the auth payload holds bytes after its signature"),
        }
    }
}

impl std::error::Error for AuthError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AuthError::Utf8(error) => Some(error),
            AuthError::Short | AuthError::NoToken | AuthError::Long => None,
        }
    }
}
