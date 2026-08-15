//! What a client's channel request frame carries for an MCP plugin.

use std::error::Error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::http::request::Request;

/// What a caller asks a provider for while a plugin runs.
///
/// A payload leads with one byte saying which — `0` for
/// [`Mcp`](Self::Mcp) — and the rest is that variant's own bytes.
///
/// It reaches INTO the container, which is the thing a caller cannot
/// dial: it runs on the provider, on a port the provider published to
/// its own loopback and told nobody. That is the whole reason this
/// channel opens outward from the client rather than the other way.
///
/// # One variant, and still an enum
///
/// A [`laboratory creation`](crate::endpoints::laboratories::create::client::channel_request::Frame)
/// has five, of which four are about files. None of those four belong
/// here — a plugin serves tools rather than holds a filesystem, takes
/// no [`mounts`], and reports no tree — so what is left is the one.
///
/// The tag byte stays anyway, for the same reason
/// [`write_path`](crate::shared::container::write_path::response::Frame)
/// spends one: a second thing to ask a running plugin is additive if
/// there is a tag to add to, and a wire break if there is not. Stopping
/// one is the obvious candidate, and it is not written.
///
/// [`mounts`]: crate::endpoints::laboratories::create::client::request::Frame::mounts
#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    /// One MCP exchange, toward the container. Tag `0`.
    ///
    /// The provider relays and nothing more. It does not parse
    /// JSON-RPC, does not track sessions, and never reads the
    /// `Mcp-Session-Id` that ties a caller's exchanges together.
    ///
    /// # It is the same relay a laboratory gets, pointed somewhere
    /// else
    ///
    /// A laboratory's MCP server was put there by the provider, so the
    /// provider chose its port. A plugin's arrived with the image and
    /// bound whatever its author chose, which the caller stated as
    /// [`port`](crate::endpoints::mcp_plugin::client::request::Frame::port).
    /// Both end up as an HTTP request written onto a socket inside the
    /// container. Nothing about the relaying differs; only what it was
    /// aimed at, and that was settled before the container started.
    ///
    /// Which means a wrong `port` surfaces HERE, as an exchange that
    /// finishes without an answer, rather than at creation — the
    /// container came up fine, and there was never anything to
    /// discover.
    Mcp(Request<'a>),
}

/// Tag for [`Frame::Mcp`].
const MCP: u8 = 0;

impl Encode for Frame<'_> {
    /// The ordinary JSON failure. The tag cannot fail.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Mcp(request) => {
                out.extend_from_slice(&[MCP]);
                request.encode(out)
            }
        }
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// Three ways to fail, and only one of them is JSON.
    type Error = FrameError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            MCP => Request::decode(rest).map(Frame::Mcp).map_err(FrameError::Mcp),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An MCP plugin channel request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag this version does not define.
    UnknownTag(u8),
    /// The MCP request did not parse.
    Mcp(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("mcp plugin channel request frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown mcp plugin channel request tag {tag}")
            }
            FrameError::Mcp(error) => {
                write!(f, "mcp request did not parse: {error}")
            }
        }
    }
}

impl Error for FrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FrameError::Mcp(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
