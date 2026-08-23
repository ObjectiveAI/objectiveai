//! What a client's response frame carries on a tool listing channel.

use std::error;
use std::fmt;

use rmcp::ErrorData;
use rmcp::model::ListToolsResult;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// The answer to a tool listing, or the reason there is not one.
///
/// The payload of a
/// [`ClientFrame::ChannelResponse`](crate::frame::client::ClientFrame::ChannelResponse)
/// on a channel opened by
/// [`channel_request::Frame::ListTools`](crate::endpoints::agentic_loop::run::server::channel_request::Frame::ListTools).
///
/// A payload leads with one byte saying which — `0` for
/// [`Result`](Self::Result), `1` for [`Error`](Self::Error) — and the
/// rest is that variant's own JSON.
///
/// # One frame, then the finish
///
/// An answer is one value, so there is one of these per channel. No
/// head, no body, and nothing to reassemble — which is the whole of
/// what replacing the tunneled HTTP exchange bought.
///
/// # The error is [`rmcp`]'s, not this crate's
///
/// Everywhere else a failure travels as
/// [`shared::error::Error`](crate::shared::error::Error), which is one
/// opaque JSON value, because everywhere else the failure is the
/// PROVIDER's own and it is nobody's business what it says.
///
/// This one is not the provider's. It is an MCP server's, relayed, and
/// its JSON-RPC code is content rather than detail: `-32601` is "no
/// such tool" and `-32602` is "the arguments were wrong", and an agent
/// told only that something failed cannot tell those apart or act
/// differently on them.
///
/// So the code goes through, along with the message and whatever data
/// came with it. A relay that flattened them would be deciding that an
/// MCP error means less than MCP says it does.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The tools a server offers, and a cursor if there are more. Tag `0`.
    Result(ListToolsResult),
    /// The server refused or could not answer. Tag `1`.
    ///
    /// See the type's own documentation for why this is
    /// [`ErrorData`] rather than the error every other endpoint uses.
    Error(ErrorData),
}

/// Tag for [`Frame::Result`].
const RESULT: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 1;

impl Encode for Frame {
    /// The ordinary JSON failure. Both variants are serialized and the
    /// tag cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Result(result) => {
                out.extend_from_slice(&[RESULT]);
                serde_json::to_writer(out, result)
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                serde_json::to_writer(out, error)
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Three ways to fail, and only one of them is JSON.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            RESULT => serde_json::from_slice(rest)
                .map(Frame::Result)
                .map_err(FrameError::Body),
            ERROR => serde_json::from_slice(rest)
                .map(Frame::Error)
                .map_err(FrameError::Body),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An answer to a tool listing that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is neither of this frame's two.
    UnknownTag(u8),
    /// The payload after the tag did not parse.
    Body(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("list_tools response frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown list_tools response frame tag {tag}")
            }
            FrameError::Body(error) => {
                write!(f, "list_tools response did not parse: {error}")
            }
        }
    }
}

impl error::Error for FrameError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            FrameError::Body(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
