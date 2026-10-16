//! What a server's response frame carries for a FUSE mount made while a container runs.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

use serde::Serialize;

/// Whether the mount the caller asked for while the container runs
/// is made.
///
/// A payload leads with one byte saying which — `0` for
/// [`Mounted`](Self::Mounted), `1` for [`Overlaps`](Self::Overlaps),
/// `2` for [`Taken`](Self::Taken), `3` for
/// [`Unsupported`](Self::Unsupported), `4` for [`Error`](Self::Error)
/// — and only the error carries anything after it.
///
/// # Exactly one, then the finish
///
/// The provider sends exactly one of these on the channel, after the
/// proxy has answered where the proxy was asked, and finishes the
/// channel. A [`Mounted`](Self::Mounted) precedes every ask the
/// provider relays for the mount's id. The mount lives until the run
/// ends, and nothing unmounts it. A finish that none of these
/// precedes is a request that did not decode.
///
/// # As JSON
///
/// For a client that prints what the provider answered: the variant,
/// snake case — `"mounted"`, `"overlaps"` — and the error over its
/// payload, `{"error":…}`. The wire is the tag byte and the payload,
/// never this.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Frame {
    /// The mount is made and serving at the path; every ask of the id
    /// follows. Tag `0`.
    Mounted,
    /// The path is a mount's the container holds, lies inside one, or
    /// holds one — a volume mount's or a FUSE mount's, the request's
    /// or one made since; nothing is mounted. Tag `1`.
    Overlaps,
    /// The id is one the container holds a FUSE mount under; nothing
    /// is mounted. Tag `2`.
    Taken,
    /// The provider holds out no `containers_fuse_mount`, and makes no
    /// FUSE mount; nothing is mounted. Tag `3`.
    Unsupported,
    /// The proxy did not make the mount: its error, verbatim. Tag `4`.
    ///
    /// Nothing is mounted. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why it
    /// says so little.
    Error(Error),
}

/// Tag for [`Frame::Mounted`].
const MOUNTED: u8 = 0;

/// Tag for [`Frame::Overlaps`].
const OVERLAPS: u8 = 1;

/// Tag for [`Frame::Taken`].
const TAKEN: u8 = 2;

/// Tag for [`Frame::Unsupported`].
const UNSUPPORTED: u8 = 3;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 4;

/// A tag, and — for the error alone — that variant's own JSON.
impl Encode for Frame {
    /// The ordinary JSON failure. The bare answers cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Mounted => {
                out.extend_from_slice(&[MOUNTED]);
                Ok(())
            }
            Frame::Overlaps => {
                out.extend_from_slice(&[OVERLAPS]);
                Ok(())
            }
            Frame::Taken => {
                out.extend_from_slice(&[TAKEN]);
                Ok(())
            }
            Frame::Unsupported => {
                out.extend_from_slice(&[UNSUPPORTED]);
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
            MOUNTED => Ok(Frame::Mounted),
            OVERLAPS => Ok(Frame::Overlaps),
            TAKEN => Ok(Frame::Taken),
            UNSUPPORTED => Ok(Frame::Unsupported),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A FUSE mount made while a container runs response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's five.
    UnknownTag(u8),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("fuse mount response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown fuse mount response frame tag {tag}"),
            FrameError::Error(error) => write!(f, "fuse mount error did not parse: {error}"),
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
