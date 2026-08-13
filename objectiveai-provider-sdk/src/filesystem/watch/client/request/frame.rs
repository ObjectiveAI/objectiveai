//! What a client's request frame carries for a watch.

use serde::{Deserialize, Serialize};

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// Watch one of the directories a provider offers.
///
/// # A name, not a path
///
/// The one field is a
/// [`Directory::name`](crate::filesystem::list::server::response::Directory::name)
/// from a listing, and this is the whole of the access model. A caller
/// cannot watch a directory it was not offered, cannot escape one by
/// naming components above it, and cannot probe for what exists by
/// watching and reading the error — because a path it invents is not
/// something this request can express.
///
/// A provider therefore never has to validate a path, only look up a
/// name it published. That is a smaller job and a much smaller
/// mistake to make.
///
/// # What comes back
///
/// A [`filetree`](crate::filetree) stream on channel `0`: one snapshot
/// carrying the whole tree, then one frame per change for as long as
/// the scope lives. Every path in it is relative to the directory
/// named here.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Frame {
    /// Which directory, by the name a listing gave it.
    ///
    /// Names come from
    /// [`Directory::name`](crate::filesystem::list::server::response::Directory::name)
    /// and mean nothing outside the provider that published them.
    pub name: String,
}

/// This frame's tag among the scope-opening requests.
///
/// `0` is the agentic loop, `1` the image check, `2` the filesystem
/// listing. The values are allocated across four modules that do not
/// know about each other, so a fifth request has to look at all of
/// them.
const TAG: u8 = 3;

/// Postcard, matching the rest of [`filesystem`](crate::filesystem)
/// and the [`filetree`](crate::filetree) stream this opens.
impl Encode for Frame {
    /// Postcard's own failure. The tag cannot fail.
    type Error = postcard::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[TAG]);
        postcard::to_io(self, &mut *out)?;
        Ok(())
    }
}

impl Decode<'_> for Frame {
    /// Three ways to fail, and only one of them is postcard's.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        if *tag != TAG {
            return Err(FrameError::UnexpectedTag(*tag));
        }
        postcard::from_bytes(rest).map_err(FrameError::Body)
    }
}

/// A watch request frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag naming some other request.
    ///
    /// A reader that dispatched on the tag will not see this. One that
    /// assumed which request it held, and was wrong, will — which is
    /// the point of checking a tag rather than skipping it.
    UnexpectedTag(u8),
    /// The request did not parse.
    Body(postcard::Error),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("watch request frame is empty")
            }
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected watch request tag {TAG}, found {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "watch request did not parse: {error}")
            }
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Body(error) => Some(error),
            FrameError::Empty | FrameError::UnexpectedTag(_) => None,
        }
    }
}
