//! What a client's channel request frame carries for a connection.

use std::error::Error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};
use crate::http::request::Request;

/// One MCP exchange, toward the container.
///
/// The same ask a creation makes, from the other side of the same
/// container: a connector reaches the MCP server inside it by opening
/// a channel, and the provider relays.
///
/// The provider relays and nothing more. It does not parse JSON-RPC,
/// does not track sessions, and never reads the `Mcp-Session-Id` that
/// ties a connector's exchanges together — so two connectors on one
/// container hold two sessions the provider has no opinion about.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame<'a>(
    /// The request, verbatim.
    pub Request<'a>,
);

/// This frame's tag among a connection's client channel requests.
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

/// A connection channel request that could not be read.
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
                f.write_str("connection channel request frame is empty")
            }
            FrameError::UnexpectedTag(tag) => {
                write!(
                    f,
                    "expected connection channel request tag {TAG}, \
                     found {tag}"
                )
            }
            FrameError::Body(error) => {
                write!(f, "connection channel request did not parse: {error}")
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
