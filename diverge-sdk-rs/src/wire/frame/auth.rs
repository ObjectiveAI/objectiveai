//! Who is dialling, and on whose word.

use std::fmt;
use std::str::{self, Utf8Error};

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// The payload of an [`Auth`](super::client::ClientFrame::Auth)
/// frame, in either direction.
///
/// One type for both ends, because a connection may be dialled from
/// either and whichever side dialled sends this. There is nothing
/// asymmetric about presenting a credential.
///
/// A payload leads with one byte saying which mode — `0` for
/// [`Unbrokered`](Self::Unbrokered) — and the rest is that mode's own
/// bytes.
///
/// # A byte for one mode
///
/// There is one mode today and the byte still ships, because there is
/// a second one coming and a tag added later is a wire break. It is a
/// synthetic header in the honest sense: nothing about an unbrokered
/// credential needs discriminating, and the byte is there for what
/// comes after it rather than for what is there now.
///
/// **Brokered** is the mode that is not here. A broker vouching for a
/// peer is a different exchange with a different shape, not yet
/// defined — and reserving tag
/// `1` for it costs nothing while guessing at it would cost the
/// design.
///
/// # There is still no answer
///
/// Whatever mode a credential arrives in, an accepted one is followed
/// by the connection simply working and a rejected one by a close. A
/// peer that has not authenticated cannot make the far end compose a
/// reply, so a bad credential costs its sender a socket and earns it
/// nothing: no bytes to amplify, and no answer to read a reason out
/// of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Auth<'a> {
    /// The two ends already know each other. Tag `0`.
    ///
    /// No third party is involved and nobody is vouching. The
    /// credential means whatever the two ends agreed it means before
    /// either of them dialled.
    ///
    /// # The credential is text
    ///
    /// A string rather than bytes, because every credential this is
    /// going to carry already is one — a bearer token, an API key, a
    /// signed assertion. Bytes made a caller pick an encoding for
    /// something that never needed one, and made a reader hold a blob
    /// it could not print.
    ///
    /// What is IN the string is not this protocol's business. It is
    /// not parsed, not validated beyond being UTF-8, and not
    /// constrained in length — a far end that cannot make sense of one
    /// closes the connection, which is the only thing it ever does
    /// about a credential it does not like.
    ///
    /// It runs to the end of the payload, so it needs no length.
    Unbrokered(&'a str),
}

/// Tag for [`Auth::Unbrokered`].
const UNBROKERED: u8 = 0;

/// A tag and a string's own bytes. No serialization: a mode byte and
/// one trailing field are not a shape a format would help with.
impl Encode for Auth<'_> {
    /// [`Infallible`](std::convert::Infallible): a known byte and a
    /// string's own.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Auth::Unbrokered(credential) => {
                out.extend_from_slice(&[UNBROKERED]);
                out.extend_from_slice(credential.as_bytes());
            }
        }
        Ok(())
    }
}

impl<'a> Decode<'a> for Auth<'a> {
    /// Three ways to fail, and only one of them is a parse.
    type Error = AuthError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        let (mode, rest) = bytes.split_first().ok_or(AuthError::Empty)?;
        match *mode {
            UNBROKERED => str::from_utf8(rest)
                .map(Auth::Unbrokered)
                .map_err(AuthError::Credential),
            mode => Err(AuthError::UnknownMode(mode)),
        }
    }
}

/// A credential that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthError {
    /// No bytes at all, so not even a mode.
    Empty,
    /// A mode this version does not define.
    ///
    /// Which is what a brokered credential will look like until
    /// brokering exists. Rejected rather than skipped: a peer
    /// authenticating in a mode this end cannot evaluate has not
    /// authenticated, and treating an unreadable credential as absent
    /// is friendlier than treating it as present.
    UnknownMode(u8),
    /// The credential was not UTF-8.
    Credential(Utf8Error),
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::Empty => f.write_str("auth payload is empty"),
            AuthError::UnknownMode(mode) => {
                write!(f, "unknown auth mode {mode}")
            }
            AuthError::Credential(error) => {
                write!(f, "credential is not utf-8: {error}")
            }
        }
    }
}

impl std::error::Error for AuthError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AuthError::Credential(error) => Some(error),
            AuthError::Empty | AuthError::UnknownMode(_) => None,
        }
    }
}
