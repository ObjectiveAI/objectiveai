//! What a client's channel request frame carries for a connection.

use std::error::Error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::http::request::Request;
use crate::shared::container::read;

/// What a connector asks a provider for while it is attached.
///
/// A payload leads with one byte saying which — `0` for
/// [`Mcp`](Self::Mcp), `1` for [`Read`](Self::Read) — and the rest is
/// that variant's own bytes.
///
/// Both reach INTO the container, which is the thing a connector
/// cannot dial: it runs on the provider. That is the whole reason
/// these channels open outward from the client rather than the other
/// way.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    /// One MCP exchange, toward the container.
    ///
    /// The provider relays and nothing more. It does not parse
    /// JSON-RPC, does not track sessions, and never reads the
    /// `Mcp-Session-Id` that ties a connector's exchanges together —
    /// so two clients on one container hold two sessions the provider
    /// has no opinion about.
    Mcp(Request<'a>),
    /// One file, read out of the container.
    ///
    /// See [`read`](crate::shared::container::read) for why this is one file and
    /// never a directory.
    Read(read::request::Request),
}

/// Tag for [`Frame::Mcp`].
const MCP: u8 = 0;

/// Tag for [`Frame::Read`].
const READ: u8 = 1;

impl Encode for Frame<'_> {
    /// The ordinary JSON failure, from whichever half is present.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Mcp(request) => {
                out.extend_from_slice(&[MCP]);
                request.encode(out)
            }
            Frame::Read(request) => {
                out.extend_from_slice(&[READ]);
                request.encode(out)
            }
        }
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// Four ways to fail, and each names which half failed.
    type Error = FrameError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            MCP => Request::decode(rest).map(Frame::Mcp).map_err(FrameError::Mcp),
            READ => read::request::Request::decode(rest)
                .map(Frame::Read)
                .map_err(FrameError::Read),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A connection channel request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is neither [`Frame::Mcp`] nor [`Frame::Read`].
    UnknownTag(u8),
    /// The MCP request did not parse.
    Mcp(serde_json::Error),
    /// The read request did not parse.
    Read(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("connection channel request frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown connection channel request tag {tag}")
            }
            FrameError::Mcp(error) => {
                write!(f, "mcp request did not parse: {error}")
            }
            FrameError::Read(error) => {
                write!(f, "read request did not parse: {error}")
            }
        }
    }
}

impl Error for FrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FrameError::Mcp(error) | FrameError::Read(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
