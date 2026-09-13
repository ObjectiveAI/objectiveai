//! What a server's channel response frame carries on a register
//! channel.

use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::error::Error;

/// The agent taken, or not.
///
/// A payload leads with one byte saying which — `0` for
/// [`Registered`](Self::Registered), `1` for [`Error`](Self::Error) —
/// and only the error carries anything after it.
///
/// # How it ends
///
/// | the channel ends with | means |
/// |-----------------------|-------|
/// | a [`Registered`](Self::Registered), then a finish | the container holds the agent, for its life |
/// | an [`Error`](Self::Error), then a finish | it does not, and this is why |
///
/// Only a finish ends it. A registration refused is a container the
/// server does not run a loop in.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The container holds the agent now, for its life. Tag `0`.
    Registered,
    /// The container refused it. Tag `1`.
    ///
    /// The agent's server's own words: a value the image will not
    /// take, or an agent already registered. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why
    /// it says so little.
    Error(Error),
}

/// Tag for [`Frame::Registered`].
const REGISTERED: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 1;

/// A tag, and — for the error alone — that variant's own JSON.
impl Encode for Frame {
    /// The ordinary JSON failure. `Registered` cannot fail at all.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(
        &self,
        out: &mut Writer<'_>,
    ) -> Result<(), serde_json::Error> {
        match self {
            Frame::Registered => {
                out.extend_from_slice(&[REGISTERED]);
                Ok(())
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out)
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Three ways to fail, and only one of them is JSON.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            REGISTERED => Ok(Frame::Registered),
            ERROR => {
                Error::decode(rest).map(Frame::Error).map_err(FrameError::Error)
            }
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A register response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is neither of this frame's two.
    UnknownTag(u8),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("register response frame is empty"),
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown register response frame tag {tag}")
            }
            FrameError::Error(error) => {
                write!(f, "register error did not parse: {error}")
            }
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
