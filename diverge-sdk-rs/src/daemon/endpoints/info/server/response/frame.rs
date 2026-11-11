//! What a server's response frame carries for an info request.

use serde::Serialize;
use std::fmt;

use super::Info;
use crate::shared::error::Error;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// An info's answer: what the daemon is, forbidden, or a failure.
///
/// One question and one reply, so there is exactly one of these per
/// scope, before the finish that ends it. A payload leads with one byte
/// saying which — `0` for [`Info`](Self::Info), `1` for
/// [`Forbidden`](Self::Forbidden), `2` for [`Error`](Self::Error) — and
/// the rest is that variant's own JSON: the [`Info`] for the first,
/// nothing for the bare answer, the error for the last.
///
/// # Only a relay refuses or fails
///
/// Served here, the answer is always [`Info`](Self::Info): the daemon
/// always knows what it is. [`Forbidden`](Self::Forbidden) and
/// [`Error`](Self::Error) are the relay's — a relay the account may not
/// use, or one that failed — as for every relayed request.
///
/// # As JSON
///
/// For a client that prints what the daemon answered: the variant,
/// snake case, over its payload — `{"info":{"version":"2.3.0"}}`,
/// `{"error":…}` — and a bare variant the string alone —
/// `"forbidden"`. The wire is the tag byte and the payload, never this.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Frame {
    /// What the daemon is: see [`Info`]. Tag `0`.
    Info(Info),
    /// The account the request is served for may not use the relay it
    /// named; nothing was asked. Tag `1`.
    Forbidden,
    /// A failure. Tag `2`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Info`].
const INFO: u8 = 0;

/// Tag for [`Frame::Forbidden`].
const FORBIDDEN: u8 = 1;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 2;

/// A tag, then the variant's own JSON, if it has any.
impl Encode for Frame {
    /// The ordinary JSON failure. The bare answer cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Info(info) => {
                out.extend_from_slice(&[INFO]);
                serde_json::to_writer(out, info)
            }
            Frame::Forbidden => {
                out.extend_from_slice(&[FORBIDDEN]);
                Ok(())
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out)
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Four ways to fail, and each names which half failed.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            INFO => serde_json::from_slice(rest).map(Frame::Info).map_err(FrameError::Info),
            FORBIDDEN => Ok(Frame::Forbidden),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An info response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's three.
    UnknownTag(u8),
    /// The info did not parse.
    Info(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("info response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown info response frame tag {tag}"),
            FrameError::Info(error) => write!(f, "info did not parse: {error}"),
            FrameError::Error(error) => write!(f, "info error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Info(error) | FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
