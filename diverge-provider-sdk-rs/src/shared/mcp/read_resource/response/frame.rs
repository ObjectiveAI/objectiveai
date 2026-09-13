//! What reading a resource produced.

use rmcp::ErrorData;
use rmcp::model::ReadResourceResult;

use super::super::super::FrameError;
use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// The resource's contents, or the reason there are none.
///
/// A payload leads with one byte saying which — `0` for
/// [`Result`](Self::Result), `1` for [`Error`](Self::Error) — and the
/// rest is that variant's own JSON.
///
/// # One frame, then the finish
///
/// A read is answered once. There is no head, no body, and nothing
/// to reassemble.
///
/// # The error is [`rmcp`]'s, not this crate's
///
/// Everywhere else in this protocol a failure travels as
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
    /// The contents. Tag `0`.
    Result(ReadResourceResult),
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
            Frame::Result(value) => {
                out.extend_from_slice(&[RESULT]);
                serde_json::to_writer(out, value)
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
