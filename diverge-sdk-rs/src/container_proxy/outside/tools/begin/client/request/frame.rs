//! What a client's request frame carries for a tool container begin.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::containers::request::Image;

/// Begin the server's work on a tool container, and hand it its
/// arguments.
///
/// The arguments are the
/// [`arguments`](crate::shared::containers::request::Container::arguments)
/// of the request that made the container, typed to the same depth
/// for the same reason — a JSON value, because the image defines what
/// it takes, and what the value may be is what
/// [`schema`](crate::shared::containers::schema) answers. They ride
/// the begin rather than a channel of their own because they are
/// handed over exactly once, first, and never change: the container
/// that has begun is a container that holds its arguments, and
/// [`Begun`](super::super::super::server::response::Frame::Begun) says both.
///
/// The image rides beside them for the proxy's own use: the proxy
/// cannot see what image it runs in, and it puts the name and the
/// digest under `_meta` on every MCP exchange it relays — see
/// [`shared::mcp`](crate::shared::mcp) — so that whoever is on the
/// other end of a tool call knows which image is calling, and which
/// image is serving.
///
/// # Once, and first
///
/// The server opens this before any other scope on the connection,
/// and never again on it: a second begin is answered
/// [`Error`](super::super::super::server::response::Frame::Error) and
/// finished, and the first goes on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    /// The arguments, as the image defines them.
    pub arguments: Value,
    /// The image the container was made from, name and digest, as the
    /// run request named it.
    pub image: Image,
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
const TAG: u8 = 1;

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

/// A tools begin request that could not be read.
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
    /// The arguments did not parse.
    Body(serde_json::Error),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("tools begin request frame is empty")
            }
            FrameError::UnexpectedTag(tag) => {
                write!(f, "expected tools begin request tag {TAG}, found {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "tools begin request did not parse: {error}")
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
