//! What a server's response frame carries for a creation.

use std::error::Error;
use std::fmt;
use std::string::FromUtf8Error;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// A creation's answer: the container's id, and its filesystem for as
/// long as the scope lives.
///
/// # The tag byte
///
/// A payload leads with one byte saying which variant it is — `0` for
/// [`Id`](Self::Id), `1` for [`Filetree`](Self::Filetree) — and the
/// rest is that variant's own bytes. The same arrangement
/// [`http::response::Frame`](crate::http::response::Frame) uses, and
/// for the same reason: a frame that means something only in the
/// context of the ones before it needs a reader carrying state, and
/// one arriving out of order is not detectably wrong, it is silently
/// the other thing.
///
/// # Which is why nothing here is ordered
///
/// The two kinds interleave however a provider produces them. An id
/// may land before the first filesystem frame, after the snapshot, or
/// somewhere in the middle of the deltas — a container can be running
/// and reporting before its provider has finished deciding what to
/// call it, and nothing is served by making one wait for the other.
///
/// A reader takes each frame as it comes and does not count. That is
/// what the tag bought, and it paid for itself here: had the split
/// been positional, an order would have had to be invented and then
/// obeyed by every provider forever.
///
/// It costs a byte per filetree frame, which is the bulk of the
/// traffic. That is the price of frames that can be read on their own,
/// and it is small next to the paths they carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame {
    /// The container's id. Tag `0`.
    ///
    /// Arrives whenever the provider has it, which is not necessarily
    /// before the filesystem starts reporting.
    ///
    /// What it is for is between the two ends. What it is FROM is this
    /// scope: a provider mints it here, and a caller that wants to
    /// name this container anywhere else has this and nothing else to
    /// name it with.
    Id(String),
    /// One change on the container's filesystem. Tag `1`.
    ///
    /// A [`filetree`](crate::filetree) stream over the container's own
    /// root — one snapshot, then one frame per change — which is the
    /// same thing
    /// [`filesystem::watch`](crate::filesystem::watch) answers with,
    /// over a different tree.
    ///
    /// Which is why creating and watching are not two asks. A
    /// container's filesystem is the observable part of it running, so
    /// the scope that made the container is the scope that reports on
    /// it.
    Filetree(crate::filetree::response::Frame),
}

/// Tag for [`Frame::Id`].
const ID: u8 = 0;

/// Tag for [`Frame::Filetree`].
const FILETREE: u8 = 1;

/// Two variants, two encodings. An id is a string and goes out as its
/// own bytes; a filetree frame is postcard's, and is handed to
/// postcard. Neither is re-encoded into the other's format to make
/// them match, because matching would buy nothing.
impl Encode for Frame {
    /// Postcard's, since only the filetree half can fail. Writing a
    /// string's bytes after a tag has no failure mode.
    type Error = postcard::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Id(id) => {
                out.extend_from_slice(&[ID]);
                out.extend_from_slice(id.as_bytes());
                Ok(())
            }
            Frame::Filetree(frame) => {
                out.extend_from_slice(&[FILETREE]);
                frame.encode(out)
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Four ways to fail, and each names which half failed.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            ID => String::from_utf8(rest.to_vec())
                .map(Frame::Id)
                .map_err(FrameError::Id),
            FILETREE => crate::filetree::response::Frame::decode(rest)
                .map(Frame::Filetree)
                .map_err(FrameError::Filetree),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A creation response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is neither [`Frame::Id`] nor [`Frame::Filetree`].
    UnknownTag(u8),
    /// The id was not UTF-8.
    Id(FromUtf8Error),
    /// The filetree frame did not decode.
    Filetree(postcard::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("creation response frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown creation response frame tag {tag}")
            }
            FrameError::Id(error) => {
                write!(f, "container id is not utf-8: {error}")
            }
            FrameError::Filetree(error) => {
                write!(f, "filetree frame did not decode: {error}")
            }
        }
    }
}

impl Error for FrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FrameError::Id(error) => Some(error),
            FrameError::Filetree(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
