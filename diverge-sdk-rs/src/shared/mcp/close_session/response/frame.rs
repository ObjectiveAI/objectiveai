//! What closing a session produced.

use rmcp::ErrorData;

use super::super::super::FrameError;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// The session is closed, or the reason it could not be.
///
/// A payload leads with one byte saying which — `0` for
/// [`Closed`](Self::Closed), `1` for [`Error`](Self::Error) — and the
/// rest is that variant's own bytes. The rules:
///
/// 1. [`Closed`](Self::Closed) is the tag alone: nothing follows it.
/// 2. [`Error`](Self::Error) is the tag and then the [`ErrorData`]'s
///    JSON, as every other MCP exchange's error is.
/// 3. One frame answers the channel, then the finish.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The session is closed, or was never open. Tag `0`.
    Closed,
    /// It could not be closed. Tag `1`.
    ///
    /// [`ErrorData`] for the reason the other MCP exchanges give: what
    /// is relayed is the MCP side's failure, not the provider's.
    Error(ErrorData),
}

/// Tag for [`Frame::Closed`].
const CLOSED: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 1;

impl Encode for Frame {
    /// The ordinary JSON failure. Only the error is serialized, and
    /// the tag cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Closed => {
                out.extend_from_slice(&[CLOSED]);
                Ok(())
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
            // Rule 1: whatever follows the tag is ignored, as an empty
            // request ignores its bytes.
            CLOSED => Ok(Frame::Closed),
            ERROR => serde_json::from_slice(rest)
                .map(Frame::Error)
                .map_err(FrameError::Body),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}
