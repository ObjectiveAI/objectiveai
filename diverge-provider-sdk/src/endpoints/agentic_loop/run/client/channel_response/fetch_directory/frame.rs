//! One file — or one chunk of one — of the directory being fetched.

use std::error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// One file's placement and bytes.
///
/// A fetched directory arrives as one of these per file, in no
/// promised order, and the channel's finish is what says the
/// directory is whole. Zero frames before the finish is the client
/// saying it does not hold the identity at all.
///
/// The body is borrowed from the frame it arrived in, the
/// [`oci`](crate::shared::oci::response::Frame) way: the receiver is
/// about to write these bytes somewhere, and copying them first
/// would double every chunk's memory for nothing. The path is owned —
/// it is parsed out of its JSON, and it is small.
///
/// # Adjacency is the chunking
///
/// A file larger than [`CHUNK_SIZE`](super::super::CHUNK_SIZE) is
/// sent as
/// consecutive frames with an EQUAL path, in order, and the receiver
/// concatenates — chunk-naive by design: same path, next frame,
/// append; a new path begins a new file. No index, no offset, no
/// "last one" marker. (A sender only splits what exceeds the chunk
/// size, so a zero-byte continuation frame cannot occur; a
/// legitimately empty file is one frame with no bytes.)
///
/// # The path is parts, and the last part is the filename
///
/// Relative to the directory being fetched — the request named the
/// directory, so the frames do not repeat it. Parts rather than a
/// joined string, because a separator convention is a thing two ends
/// can disagree about and an array is not.
///
/// # Three parts on the wire
///
/// `[u32 BE: byte length of the path JSON][path, a JSON array of
/// strings][everything after: the bytes, verbatim]`. The body is raw
/// rather than JSON because a file's bytes are not anybody's
/// document — encoding them would mean base64 and a third more wire
/// for nothing.
///
/// # A short set is detectable, and that is enough
///
/// A client that dies mid-directory leaves the server with some
/// frames and a finish it cannot tell from completion. No frame says
/// "last one" — the identity does: the server hashes and measures
/// what arrived, and a partial set fails both.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Frame<'a> {
    /// The file's path relative to the fetched directory,
    /// one component per element, the final element the filename.
    pub path: Vec<String>,
    /// The bytes — this frame's chunk of them — borrowed from the
    /// frame they arrived in.
    pub body: &'a [u8],
}

/// The bytes the path length occupies.
const PATH_LEN: usize = 4;

impl Encode for Frame<'_> {
    type Error = EncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), EncodeError> {
        let path =
            serde_json::to_vec(&self.path).map_err(EncodeError::Path)?;
        let len = u32::try_from(path.len())
            .map_err(|_| EncodeError::PathLength(path.len()))?;
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(&path);
        out.extend_from_slice(self.body);
        Ok(())
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    type Error = FrameError;

    fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        if bytes.len() < PATH_LEN {
            return Err(FrameError::Short(bytes.len()));
        }
        let (len, rest) = bytes.split_at(PATH_LEN);
        let len = u32::from_be_bytes(
            <[u8; PATH_LEN]>::try_from(len).expect("split_at gave 4 bytes"),
        ) as usize;
        if rest.len() < len {
            return Err(FrameError::Truncated {
                need: len,
                have: rest.len(),
            });
        }
        let (path, body) = rest.split_at(len);
        Ok(Frame {
            path: serde_json::from_slice(path).map_err(FrameError::Path)?,
            body,
        })
    }
}

/// A file frame that could not be written.
#[derive(Debug)]
pub enum EncodeError {
    /// The path would not serialize.
    Path(serde_json::Error),
    /// The path's JSON is longer than the four-byte length can say.
    /// Carried rather than truncated, because a length that lies is a
    /// frame that reads somebody else's bytes as its own.
    PathLength(usize),
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EncodeError::Path(error) => {
                write!(f, "fetch file path did not serialize: {error}")
            }
            EncodeError::PathLength(len) => {
                write!(
                    f,
                    "fetch file path is {len} bytes of JSON, more than a \
                     u32 can say"
                )
            }
        }
    }
}

impl error::Error for EncodeError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            EncodeError::Path(error) => Some(error),
            EncodeError::PathLength(_) => None,
        }
    }
}

/// A file frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// Fewer bytes than the path length itself, carrying however many
    /// there were.
    Short(usize),
    /// A path length pointing past the end of the frame.
    Truncated {
        /// What the length claimed.
        need: usize,
        /// What was actually there.
        have: usize,
    },
    /// The path did not parse as a JSON array of strings.
    Path(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Short(len) => {
                write!(
                    f,
                    "fetch file frame is {len} bytes, not even a path \
                     length"
                )
            }
            FrameError::Truncated { need, have } => {
                write!(
                    f,
                    "fetch file path claims {need} bytes and {have} follow"
                )
            }
            FrameError::Path(error) => {
                write!(f, "fetch file path did not parse: {error}")
            }
        }
    }
}

impl error::Error for FrameError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            FrameError::Path(error) => Some(error),
            FrameError::Short(_) | FrameError::Truncated { .. } => None,
        }
    }
}
