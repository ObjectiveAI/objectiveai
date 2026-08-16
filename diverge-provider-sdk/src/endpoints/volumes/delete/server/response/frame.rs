//! What a server's response frame carries for a volume deletion.

use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::error::Error;

/// The volume is gone, or it is not.
///
/// One of these on channel `0`, then the scope finishes. A payload
/// leads with one byte saying which — `0` for [`Deleted`](Self::Deleted), `1`
/// for [`Error`](Self::Error) — and for the first there is nothing
/// after it, because saying so IS the whole message.
///
/// | the scope ends with | means |
/// |----------------------|-------|
/// | [`Deleted`](Self::Deleted), then a finish | the volume is gone and a listing will not show it |
/// | an [`Error`](Self::Error), then a finish | it is not, and a caller should assume it is intact |
///
/// # Gone means gone, not emptied
///
/// The volume itself no longer exists. A caller that wanted the space
/// back with the name kept deletes and creates — which is two asks
/// because they are two things, and a provider that emptied one in
/// place would be doing something this frame has no way to distinguish
/// from the other.
///
/// # There is no partial deletion to report
///
/// A provider either finishes destroying the volume or reports an
/// error. What it leaves behind on failure is its own business and not
/// a state a caller can observe: a
/// [`list`](crate::endpoints::volumes::list) shows the volume or it
/// does not, and that is the only answer this protocol offers about
/// whether something exists.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The volume is gone. Tag `0`.
    Deleted,
    /// A failure. Tag `1`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Deleted`].
const DELETED: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 1;

/// A tag, and for a failure the JSON after it. Postcard encodes the
/// rest of [`volumes`](crate::endpoints::volumes) and encodes nothing
/// here — an [`Error`](Frame::Error) is a
/// [`serde_json::Value`], which deserializes through
/// `deserialize_any` and so cannot come back out of a format with no
/// self-description. The tag chooses the format, one variant at a
/// time.
impl Encode for Frame {
    /// The ordinary JSON failure, from the half that has one. A lone
    /// tag byte cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(
        &self,
        out: &mut Writer<'_>,
    ) -> Result<(), serde_json::Error> {
        match self {
            Frame::Deleted => {
                out.extend_from_slice(&[DELETED]);
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
    /// Three ways to fail, and only one of them is a parse.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            DELETED => Ok(Frame::Deleted),
            ERROR => {
                Error::decode(rest).map(Frame::Error).map_err(FrameError::Error)
            }
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A volume deletion result that could not be read.
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
            FrameError::Empty => {
                f.write_str("volume deletion result frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown volume deletion result frame tag {tag}")
            }
            FrameError::Error(error) => {
                write!(f, "volume deletion error did not parse: {error}")
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
