//! What a runner answers on an authorize channel.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Yes or no.
///
/// The whole answer to an
/// [`Authorize`](super::super::request::Authorize), and one frame is
/// all there is — this is not a stream, and a channel carrying one of
/// these finishes immediately after.
///
/// # The layout
///
/// ```text
/// [0]  denied
/// [1]  authorized
/// ```
///
/// One byte, nothing after it. A yes used to carry a nickname so the
/// runner could tell WHICH connector later left; nothing reports a
/// departure any more, so nothing has a use for the name.
///
/// # Why the answer says nothing else
///
/// A reason would have to mean something to the provider, and the
/// provider did not write the question. What was asked is between the
/// two ends; the only part this layer needs is whether the answer was
/// yes, because that is the part a provider acts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Frame {
    /// The connector may not attach.
    Denied,
    /// The connector may attach.
    Authorized,
}

/// The tag for a denial.
const DENIED: u8 = 0;

/// The tag for an authorization.
const AUTHORIZED: u8 = 1;

/// One byte, laid out by hand. Two tags are not a shape a format would
/// help with.
impl Encode for Frame {
    /// [`Infallible`](std::convert::Infallible): a known byte.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Denied => out.extend_from_slice(&[DENIED]),
            Frame::Authorized => out.extend_from_slice(&[AUTHORIZED]),
        }
        Ok(())
    }
}

impl Decode<'_> for Frame {
    /// Two ways to fail, and neither is a parse.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, _) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            DENIED => Ok(Frame::Denied),
            AUTHORIZED => Ok(Frame::Authorized),
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
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("authorization answer frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown authorization answer tag {tag}")
            }
        }
    }
}

impl std::error::Error for FrameError {}
