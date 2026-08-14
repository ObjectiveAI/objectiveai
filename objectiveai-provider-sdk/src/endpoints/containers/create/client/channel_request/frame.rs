//! What a client's channel request frame carries for a creation.

use std::error::Error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::shared::http::request::Request;

/// One MCP exchange, toward the container.
///
/// A container that runs an MCP server is reachable only from the
/// provider that runs it. So a caller opens a channel, the provider
/// relays into the container, and what comes back is a
/// [`channel_response::mcp::Frame`](crate::endpoints::containers::create::server::channel_response::mcp::Frame).
///
/// # The provider is a relay here too
///
/// It does not parse JSON-RPC, does not track sessions, and does not
/// know what a tool is. `Mcp-Session-Id` ties a caller's exchanges
/// together and the provider never reads it — the same arrangement
/// that carries MCP the other way in an agentic loop, and for the same
/// reason: a relay that parsed what it carried could only fail on what
/// its schema was too old to know.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame<'a>(
    /// The request, verbatim.
    pub Request<'a>,
);

/// This frame's tag among a creation's client channel requests.
///
/// The only one so far. Carried anyway, so a second kind of ask is a
/// new tag rather than a new frame type.
const TAG: u8 = 0;

impl Encode for Frame<'_> {
    /// The ordinary JSON failure. The tag cannot fail.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[TAG]);
        self.0.encode(out)
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// Three ways to fail, and only one of them is JSON.
    type Error = FrameError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        if *tag != TAG {
            return Err(FrameError::UnexpectedTag(*tag));
        }
        Request::decode(rest).map(Frame).map_err(FrameError::Body)
    }
}

/// A creation client channel request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag naming some other request.
    UnexpectedTag(u8),
    /// The request did not parse.
    Body(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("creation client channel request frame is empty")
            }
            FrameError::UnexpectedTag(tag) => {
                write!(
                    f,
                    "expected creation client channel request tag {TAG}, \
                     found {tag}"
                )
            }
            FrameError::Body(error) => {
                write!(
                    f,
                    "creation client channel request did not parse: {error}"
                )
            }
        }
    }
}

impl Error for FrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FrameError::Body(error) => Some(error),
            FrameError::Empty | FrameError::UnexpectedTag(_) => None,
        }
    }
}
