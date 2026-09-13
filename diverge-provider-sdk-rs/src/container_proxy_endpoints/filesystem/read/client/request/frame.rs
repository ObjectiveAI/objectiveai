//! What a client's request frame carries for a read.

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::containers::read;

/// One file, read out of the container.
///
/// The [`read::request::Request`] the caller sent the provider,
/// carried the last hop: the path, as components from the container's
/// root. See [`read`](crate::shared::containers::read) for why this is
/// one file and never a directory.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame(
    /// Which file.
    pub read::request::Request,
);

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
const TAG: u8 = 4;

/// The tag, then the request's own JSON.
impl Encode for Frame {
    /// The ordinary JSON failure. The tag cannot fail.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[TAG]);
        self.0.encode(out)
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
        read::request::Request::decode(rest)
            .map(Frame)
            .map_err(FrameError::Body)
    }
}

/// A read request that could not be read.
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
            FrameError::Empty => f.write_str("read request frame is empty"),
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected read request tag {TAG}, found {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "read request did not parse: {error}")
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
