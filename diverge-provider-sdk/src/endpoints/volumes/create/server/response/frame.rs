//! What a server's response frame carries for a volume creation.

use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::error::Error;

/// The volume exists, or it does not.
///
/// One of these on channel `0`, then the scope finishes. A payload
/// leads with one byte saying which — `0` for [`Created`](Self::Created), `1`
/// for [`Error`](Self::Error) — and for the first there is nothing
/// after it, because saying so IS the whole message.
///
/// | the scope ends with | means |
/// |----------------------|-------|
/// | [`Created`](Self::Created), then a finish | the volume exists and a listing will show it |
/// | an [`Error`](Self::Error), then a finish | it does not, and nothing partial does |
///
/// # Why it does not answer with the volume
///
/// Because the caller already knows both fields. It chose the
/// [`name`](crate::endpoints::volumes::create::client::request::Frame::name),
/// and a
/// [`created`](crate::endpoints::volumes::list::server::response::Volume::created)
/// it can predict to the second is not news. Sending a
/// [`Volume`](crate::endpoints::volumes::list::server::response::Volume)
/// back would be echoing a request with a timestamp stapled to it, and
/// a caller that wants the canonical record asks for a
/// [`list`](crate::endpoints::volumes::list) — which is the same
/// answer every other caller gets, rather than a second version of the
/// truth minted here.
///
/// # The scope ends, and the volume does not
///
/// Unlike a [`container`](crate::endpoints::containers), whose
/// scope IS its life. A volume outlives the request
/// that made it and every connection the caller ever holds; it goes
/// away when a
/// [`delete`](crate::endpoints::volumes::delete) says so and not
/// before.
///
/// Which is what makes it worth having. A caller mounts one into a
/// laboratory, the laboratory stops, and the work is still there.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The volume exists. Tag `0`.
    Created,
    /// A failure. Tag `1`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Created`].
const CREATED: u8 = 0;

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
            Frame::Created => {
                out.extend_from_slice(&[CREATED]);
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
            CREATED => Ok(Frame::Created),
            ERROR => {
                Error::decode(rest).map(Frame::Error).map_err(FrameError::Error)
            }
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A volume creation result that could not be read.
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
                f.write_str("volume creation result frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown volume creation result frame tag {tag}")
            }
            FrameError::Error(error) => {
                write!(f, "volume creation error did not parse: {error}")
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
