//! A write's content, arriving.

use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::error::Error;

/// A piece of the file being written, or the news that there will not
/// be one.
///
/// A payload leads with one byte saying which — `0` for
/// [`Body`](Self::Body), `1` for [`Error`](Self::Error) — and the rest
/// is that variant's own bytes.
///
/// # The end, the failure, and the giving up
///
/// Three ways this stream stops, and they are not the same.
///
/// | the channel ends with | means |
/// |-----------------------|-------|
/// | bodies, then a finish | that was the whole content |
/// | an [`Error`](Self::Error), then a finish | the client cannot supply it, and said why |
/// | bodies, then nothing | the client abandoned the write |
///
/// The content ends when the channel finishes. There is no terminator
/// in the payload because the frame layer already has one, and a
/// second would be two signals for one fact.
///
/// # Why abandonment survives
///
/// A client that decides mid-stream not to go through with the write
/// can still simply never finish. The provider sees content stop
/// without an end, discards the temporary, and the destination is left
/// untouched.
///
/// That is not the same as an [`Error`](Self::Error), and adding one
/// did not replace it. An error is a client that knows it cannot
/// continue and can say so; abandonment is a client that stopped,
/// which includes every case where it was not able to say anything —
/// a process that died, a pipe that broke, a network that went away.
/// A provider has to handle a stream that just stops regardless, so
/// the error is a courtesy rather than a replacement.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    /// The bytes, borrowed from the frame they arrived in. Tag `0`.
    Body(&'a [u8]),
    /// A failure. Tag `1`.
    ///
    /// The client cannot supply the content it was asked for — the
    /// source went away, the read it was piping stopped, whatever it
    /// knows. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why
    /// it says so little.
    Error(Error),
}

/// Tag for [`Frame::Body`].
const BODY: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 1;

impl Encode for Frame<'_> {
    /// The ordinary JSON failure, from the half that has one. Copying
    /// a slice into a buffer cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(
        &self,
        out: &mut Writer<'_>,
    ) -> Result<(), serde_json::Error> {
        match self {
            Frame::Body(bytes) => {
                out.extend_from_slice(&[BODY]);
                out.extend_from_slice(bytes);
                Ok(())
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out)
            }
        }
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// Three ways to fail, and only one of them is a parse.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            BODY => Ok(Frame::Body(rest)),
            ERROR => {
                Error::decode(rest).map(Frame::Error).map_err(FrameError::Error)
            }
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A write content frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    ///
    /// Distinct from a zero-length body, which is a tag followed by
    /// nothing and is ordinary on a stream.
    Empty,
    /// A tag that is neither of this frame's two.
    UnknownTag(u8),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("write content frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown write content frame tag {tag}")
            }
            FrameError::Error(error) => {
                write!(f, "write content error did not parse: {error}")
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
