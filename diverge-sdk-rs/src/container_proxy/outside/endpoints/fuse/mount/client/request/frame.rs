//! What a client's request frame carries for a FUSE mount.

use serde::{Deserialize, Serialize};

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::containers::fuse::Kind;

/// Make one FUSE mount, as the container request named it.
///
/// One [`FuseMount`](crate::shared::containers::request::FuseMount) of
/// the request, with which list it was on made explicit as the
/// [`kind`](Self::kind). The path is components from the container's
/// root, never empty, no component empty or `.` or `..`.
///
/// # The id stays with the server
///
/// A [`FuseMount`](crate::shared::containers::request::FuseMount)
/// carries the caller's id for the mount, and this does not: the scope
/// this opens IS the mount, every ask the mount makes rides it, and
/// the server — which relays each ask to the caller as the provider
/// protocol's own — knows the scope by the id and puts it back. A
/// container cannot name a mount the caller did not make, because it
/// never holds an id at all.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Frame {
    /// Where the mount goes, as components from the container's
    /// root.
    pub path: Vec<String>,
    /// One regular file, or a directory tree.
    pub kind: Kind,
}

/// This frame's tag among the scope-opening requests.
///
/// One byte at the front of the payload, which is what tells a reader
/// which request it holds. The frame layer does not discriminate them
/// — [`ClientFrame::Request`](crate::wire::frame::client::ClientFrame::Request)
/// is one type carrying bytes — so the distinction has to be in the
/// bytes, and each request owns the value that names it.
///
/// See the table in
/// [`container_proxy_endpoints`](crate::container_proxy::outside) for
/// the whole allocation. The values are chosen across modules that do
/// not know about each other, so the table is the only place they can
/// be seen at once.
const TAG: u8 = 2;

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

/// A fuse mount request that could not be read.
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
                f.write_str("fuse mount request frame is empty")
            }
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected fuse mount request tag {TAG}, found {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "fuse mount request did not parse: {error}")
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
