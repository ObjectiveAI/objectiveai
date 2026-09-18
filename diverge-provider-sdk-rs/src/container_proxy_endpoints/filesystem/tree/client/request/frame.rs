//! What a client's request frame carries for a tree.

use serde::{Deserialize, Serialize};

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// Watch the container's tree, leaving these paths out.
///
/// The server names the FUSE MOUNTS here — the ones it placed in the
/// container that the caller serves itself, so that a watch of one
/// would report the caller's own answers back to it, and every
/// mount of a volume the provider watches itself and merges into the
/// tree it sends the caller. Every other volume mount is not listed,
/// and is in the tree. `/proc`,
/// `/sys` and `/dev` are the proxy's own and are never listed. Each
/// path is components from the container's root, the shape every path
/// in this crate takes; an empty one is dropped rather than read as
/// the root. An ignored path does not exist as far as the stream is
/// concerned: absent from the snapshot, never watched, an event under
/// it dropped.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Frame {
    /// The paths to leave out, each as components from the root.
    pub ignore: Vec<Vec<String>>,
}

/// This frame's tag among the scope-opening requests.
///
/// One byte at the front of the payload, which is what tells a reader
/// which request it holds. The frame layer does not discriminate them
/// — [`ClientFrame::Request`](crate::frame::client::ClientFrame::Request)
/// is one type carrying bytes — so the distinction has to be in the
/// bytes, and each request owns the value that names it.
///
/// See the table in
/// [`container_proxy_endpoints`](crate::container_proxy_endpoints) for
/// the whole allocation. The values are chosen across modules that do
/// not know about each other, so the table is the only place they can
/// be seen at once.
const TAG: u8 = 3;

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

/// A tree request that could not be read.
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
            FrameError::Empty => {
                f.write_str("tree request frame is empty")
            }
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected tree request tag {TAG}, found {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "tree request did not parse: {error}")
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
