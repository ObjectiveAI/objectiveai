//! What a client's channel request frame carries for a laboratory run.

use std::error::Error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::container::{read, transfer, write_path};
use crate::shared::http::request::Request;

/// What a caller asks a provider for while a laboratory runs.
///
/// A payload leads with one byte saying which — `0` for
/// [`Mcp`](Self::Mcp), `1` for [`Read`](Self::Read), `2` for
/// [`Write`](Self::Write), `3` for [`Transfer`](Self::Transfer), `4`
/// for [`Stop`](Self::Stop) — and the rest is that variant's own
/// bytes, of which the last has none.
///
/// The first four reach INTO the container, which is the thing a
/// caller cannot dial: it runs on the provider. That is the whole
/// reason these channels open outward from the client rather than the
/// other way.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    /// One MCP exchange, toward the container.
    ///
    /// The provider relays and nothing more. It does not parse
    /// JSON-RPC, does not track sessions, and never reads the
    /// `Mcp-Session-Id` that ties a caller's exchanges together —
    /// so two clients on one container hold two sessions the provider
    /// has no opinion about.
    Mcp(Request<'a>),
    /// One file, read out of the container.
    ///
    /// See [`read`](crate::shared::container::read) for why this is
    /// one file and never a directory.
    Read(read::request::Request),
    /// One file, written into the container.
    ///
    /// Carries no content. The provider answers by opening a channel
    /// of its own asking for it — see
    /// [`write_path`](crate::shared::container::write_path) for why it
    /// travels that direction, and
    /// [`write_bytes`](crate::shared::container::write_bytes) for what
    /// comes back.
    Write(write_path::request::Request),
    /// One file, moved out of this container into another without
    /// ever leaving the provider.
    ///
    /// Carries no content and receives none. See
    /// [`transfer`](crate::shared::container::transfer) for when this
    /// is available and what to do when it is not.
    Transfer(transfer::request::Request),
    /// Stop the container. Tag `4`.
    ///
    /// # It has no answer, and does not need one
    ///
    /// Nothing comes back on this channel. What comes back is the end
    /// of the SCOPE — a
    /// [`ResponseFinish`](crate::frame::server::ServerFrame::ResponseFinish),
    /// which already means nothing bearing this scope follows on any
    /// channel. Finishing this one first would be a smaller way of
    /// saying the same thing, moments earlier.
    ///
    /// # What it adds over closing the connection
    ///
    /// The scope IS the container's life, so dropping the connection
    /// stops it too. The difference is that a provider cannot tell a
    /// deliberate exit from a network that stopped answering, and has
    /// to wait to find out. This is unambiguous and immediate: a
    /// caller that says so is not gone, it is finished.
    ///
    /// # What it does to everyone else
    ///
    /// Ends them. Connectors hold scopes on a container that no longer
    /// exists, so those scopes finish too — a connection cannot
    /// outlive the thing it joined.
    Stop,
}

/// Tag for [`Frame::Mcp`].
const MCP: u8 = 0;

/// Tag for [`Frame::Read`].
const READ: u8 = 1;

/// Tag for [`Frame::Write`].
const WRITE: u8 = 2;

/// Tag for [`Frame::Transfer`].
const TRANSFER: u8 = 3;

/// Tag for [`Frame::Stop`].
const STOP: u8 = 4;

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
            Frame::Write(request) => {
                out.extend_from_slice(&[WRITE]);
                request.encode(out)
            }
            Frame::Transfer(request) => {
                out.extend_from_slice(&[TRANSFER]);
                request.encode(out)
            }
            Frame::Stop => {
                out.extend_from_slice(&[STOP]);
                Ok(())
            }
        }
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// Six ways to fail, and the parses among them name which half
    /// failed.
    type Error = FrameError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            MCP => Request::decode(rest).map(Frame::Mcp).map_err(FrameError::Mcp),
            READ => read::request::Request::decode(rest)
                .map(Frame::Read)
                .map_err(FrameError::Read),
            WRITE => write_path::request::Request::decode(rest)
                .map(Frame::Write)
                .map_err(FrameError::Write),
            TRANSFER => transfer::request::Request::decode(rest)
                .map(Frame::Transfer)
                .map_err(FrameError::Transfer),
            STOP => Ok(Frame::Stop),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A laboratory run channel request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's five.
    UnknownTag(u8),
    /// The MCP request did not parse.
    Mcp(serde_json::Error),
    /// The read request did not parse.
    Read(serde_json::Error),
    /// The write request did not parse.
    Write(serde_json::Error),
    /// The transfer request did not parse.
    Transfer(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("laboratory run channel request frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown laboratory run channel request tag {tag}")
            }
            FrameError::Mcp(error) => {
                write!(f, "mcp request did not parse: {error}")
            }
            FrameError::Read(error) => {
                write!(f, "read request did not parse: {error}")
            }
            FrameError::Write(error) => {
                write!(f, "write request did not parse: {error}")
            }
            FrameError::Transfer(error) => {
                write!(f, "transfer request did not parse: {error}")
            }
        }
    }
}

impl Error for FrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FrameError::Mcp(error)
            | FrameError::Read(error)
            | FrameError::Write(error)
            | FrameError::Transfer(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
