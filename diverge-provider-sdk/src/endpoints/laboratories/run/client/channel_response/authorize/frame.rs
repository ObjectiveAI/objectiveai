//! What a client's response frame carries on an authorize channel.

use std::fmt;
use std::str::{self, Utf8Error};

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// Yes or no, and if yes, what to call it.
///
/// The whole answer to an
/// [`Authorize`](crate::endpoints::laboratories::run::server::channel_request::Frame::Authorize),
/// and one frame is all there is — this is not a stream, and a channel
/// carrying one of these finishes immediately after.
///
/// # The layout
///
/// ```text
/// [0]                      denied
/// [1][0]                   authorized, unnamed
/// [1][1][nickname: utf-8]  authorized, named
/// ```
///
/// The nickname runs to the end of the payload, so it needs no length.
/// The byte before it is presence rather than length: an empty
/// nickname and no nickname are different answers, and a layout where
/// they encode identically would be a layout that loses one of them.
///
/// A denial carries nothing after its tag. There is nothing to call
/// something that is not being let in.
///
/// # Why the answer says nothing else
///
/// A reason would have to mean something to the provider, and the
/// provider did not write the question. What was asked is between the
/// two ends; the only part this layer needs is whether the answer was
/// yes, because that is the part a provider acts on.
///
/// A caller that wants to explain itself has somewhere better to do it
/// than a channel whose whole purpose is to unblock something.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Frame<'a> {
    /// The connector may not attach.
    Denied,
    /// The connector may attach, under this name.
    ///
    /// # The name is for the runner's own benefit
    ///
    /// It is what a
    /// [`Disconnected`](crate::endpoints::laboratories::run::server::response::Frame::Disconnected)
    /// carries when this connector goes away, and that is its entire
    /// purpose: a runner that authorized four connectors has no other
    /// way to learn WHICH one left. The provider stores it and hands
    /// it back; it never reads it, and the connector is never told it
    /// has one.
    ///
    /// `None` is a runner that does not care to tell them apart. A
    /// disconnection then arrives unnamed, which is honest — nothing
    /// downstream has to invent an identity nobody supplied.
    ///
    /// Nothing requires it to be unique. Two connectors may share a
    /// nickname, and a runner that lets them has decided it does not
    /// need to distinguish those two.
    Authorized(Option<&'a str>),
}

/// The tag for a denial.
const DENIED: u8 = 0;

/// The tag for an authorization.
const AUTHORIZED: u8 = 1;

/// The byte for a nickname that is not there.
const ABSENT: u8 = 0;

/// The byte for a nickname that follows.
const PRESENT: u8 = 1;

/// Bytes laid out by hand, and no serialization. Two tags, a presence
/// byte and a string are not a shape a format would help with — and
/// the string is the last field, so nothing needs a length.
impl Encode for Frame<'_> {
    /// [`Infallible`](std::convert::Infallible): known bytes and a
    /// string's own.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Denied => out.extend_from_slice(&[DENIED]),
            Frame::Authorized(None) => {
                out.extend_from_slice(&[AUTHORIZED, ABSENT]);
            }
            Frame::Authorized(Some(nickname)) => {
                out.extend_from_slice(&[AUTHORIZED, PRESENT]);
                out.extend_from_slice(nickname.as_bytes());
            }
        }
        Ok(())
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// Four ways to fail, and only one of them is a parse.
    type Error = FrameError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            DENIED => Ok(Frame::Denied),
            AUTHORIZED => {
                let (presence, nickname) =
                    rest.split_first().ok_or(FrameError::Truncated)?;
                match *presence {
                    ABSENT => Ok(Frame::Authorized(None)),
                    PRESENT => str::from_utf8(nickname)
                        .map(|nickname| Frame::Authorized(Some(nickname)))
                        .map_err(FrameError::Nickname),
                    presence => Err(FrameError::UnknownPresence(presence)),
                }
            }
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An authorization answer that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is neither yes nor no.
    ///
    /// Rejected rather than read as truthy. Anything other than the
    /// two defined values means the sender and this reader disagree
    /// about the protocol, and guessing which way a disagreement leans
    /// is a poor way to decide an authorization.
    UnknownTag(u8),
    /// An authorization that ended before saying whether a nickname
    /// followed.
    Truncated,
    /// A presence byte that is neither absent nor present.
    UnknownPresence(u8),
    /// The nickname was not UTF-8.
    Nickname(Utf8Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("authorization answer frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(
                    f,
                    "authorization answer is neither {DENIED} nor \
                     {AUTHORIZED}: {tag}"
                )
            }
            FrameError::Truncated => {
                f.write_str("authorization answer ended before its nickname")
            }
            FrameError::UnknownPresence(presence) => {
                write!(
                    f,
                    "nickname presence is neither {ABSENT} nor {PRESENT}: \
                     {presence}"
                )
            }
            FrameError::Nickname(error) => {
                write!(f, "nickname is not utf-8: {error}")
            }
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Nickname(error) => Some(error),
            FrameError::Empty
            | FrameError::UnknownTag(_)
            | FrameError::Truncated
            | FrameError::UnknownPresence(_) => None,
        }
    }
}
