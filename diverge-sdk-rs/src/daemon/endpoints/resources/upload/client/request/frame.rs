//! What a client's request frame carries for an upload.

use serde::{Deserialize, Serialize};

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// Ask the daemon to hold a resource: a file, or a directory of the
/// files named.
///
/// JSON-tagged by `kind`. A [`File`](Self::File) carries its
/// description and nothing more: the daemon opens one content
/// channel, and the file's bytes answer it. A
/// [`Directory`](Self::Directory) names every file in it, by path
/// from the directory's root, `/`-separated, no component empty, `.`
/// or `..`, no path twice, and at least one; the daemon opens one
/// content channel per path. No name is chosen here: the resource's
/// id is its hash, which the daemon answers once it holds the bytes.
///
/// # The description is the caller's, not the content's
///
/// A description says what the resource is for, in words, for
/// whoever reads a listing. It is not in the hash: the same bytes
/// uploaded with two descriptions are one resource, and the
/// description the daemon keeps is the latest upload's, so an upload
/// answered [`Exists`](crate::daemon::endpoints::resources::upload::server::response::Frame::Exists)
/// has still said what the resource is.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Frame {
    /// One file.
    File {
        /// What the resource is, in words. Required; the daemon
        /// compares it to nothing and reads it for nothing.
        description: String,
    },
    /// One directory of files.
    Directory {
        /// What the resource is, in words. Required; the daemon
        /// compares it to nothing and reads it for nothing.
        description: String,
        /// The paths of its files, from the directory's root.
        files: Vec<String>,
    },
}

/// This frame's tag among the scope-opening requests.
///
/// One byte at the front of the payload, which is what tells a reader
/// which request it holds. The frame layer does not discriminate them
/// — [`ClientFrame::Request`](crate::wire::frame::client::ClientFrame::Request)
/// is one type carrying bytes — so the distinction has to be in the
/// bytes, and each request owns the value that names it.
///
/// See the table in [`endpoints`](crate::daemon::endpoints) for the whole
/// allocation. The values are chosen across modules that do not know
/// about each other, so the table is the only place they can be seen
/// at once.
const TAG: u8 = 34;

/// JSON, as every request of the daemon's is.
impl Encode for Frame {
    /// The ordinary JSON failure. The tag cannot fail.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[TAG]);
        serde_json::to_writer(out, self)
    }
}

impl Decode<'_> for Frame {
    /// Three ways to fail, and only one of them is JSON.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        if *tag != TAG {
            return Err(FrameError::UnexpectedTag(*tag));
        }
        serde_json::from_slice(rest).map_err(FrameError::Body)
    }
}

/// A resources upload request frame that could not be read.
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
    Body(serde_json::Error),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Empty => f.write_str("resources upload request frame is empty"),
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected resources upload request tag {TAG}, found {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "resources upload request did not parse: {error}")
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
