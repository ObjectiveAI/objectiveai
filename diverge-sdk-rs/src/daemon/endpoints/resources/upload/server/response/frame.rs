//! What a server's response frame carries for an upload.

use std::fmt;

use crate::shared::error::Error;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// An upload's answer: the resource is held, under this id; the same
/// bytes were held before, and this is their id; or a failure.
///
/// An upload is one question and one reply, sent once every content
/// channel has finished, so there is exactly one of these per scope,
/// before the finish that ends it. A payload leads
/// with one byte saying which — `0` for [`Uploaded`](Self::Uploaded),
/// `1` for [`Exists`](Self::Exists), `2` for [`Error`](Self::Error) —
/// and the rest is that variant's own JSON: the id as a JSON string
/// for the first two, the error for the third.
///
/// # Exists is not a failure
///
/// [`Exists`](Self::Exists) is an ANSWER: the caller holds exactly
/// these bytes already, and the daemon kept nothing new but the
/// request's description, which is the resource's from then on — the
/// id answered is the one the caller has, and nothing is retried. An
/// [`Error`](Self::Error) is the absence of an answer: the resource
/// could not be held — a content channel ended in an error, a path
/// was never finished, whatever the daemon knows — and nothing is
/// held.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The resource is held, and this is its id. Tag `0`.
    Uploaded(String),
    /// The same bytes were held before, and this is their id; nothing
    /// changed but the description, which is now the request's. Tag
    /// `1`.
    Exists(String),
    /// A failure. Tag `2`.
    ///
    /// Nothing is held. See
    /// [`shared::error::Error`](crate::shared::error::Error)
    /// for why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Uploaded`].
const UPLOADED: u8 = 0;

/// Tag for [`Frame::Exists`].
const EXISTS: u8 = 1;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 2;

/// A tag, then the variant's own JSON.
impl Encode for Frame {
    /// The ordinary JSON failure.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Uploaded(id) => {
                out.extend_from_slice(&[UPLOADED]);
                serde_json::to_writer(out, id)
            }
            Frame::Exists(id) => {
                out.extend_from_slice(&[EXISTS]);
                serde_json::to_writer(out, id)
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out)
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Four ways to fail, and each names which half failed.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            UPLOADED => serde_json::from_slice(rest).map(Frame::Uploaded).map_err(FrameError::Id),
            EXISTS => serde_json::from_slice(rest).map(Frame::Exists).map_err(FrameError::Id),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An upload response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's three.
    UnknownTag(u8),
    /// The id did not parse as a JSON string.
    Id(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("resources upload response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown resources upload response frame tag {tag}"),
            FrameError::Id(error) => write!(f, "resources upload id did not parse: {error}"),
            FrameError::Error(error) => write!(f, "resources upload error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Id(error) | FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
