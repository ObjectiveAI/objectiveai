//! What a schema channel answers.

use std::fmt;

use serde_json::Value;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// The arguments' schema, or the news that there is none.
///
/// A payload leads with one byte saying which — `0` for
/// [`Schema`](Self::Schema), `1` for [`Error`](Self::Error) — and the
/// rest is that variant's own JSON. One frame is all there is, and the
/// channel finishes after it.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The JSON Schema of the
    /// [`arguments`](crate::shared::containers::request::Container::arguments),
    /// as the image states it. Tag `0`.
    ///
    /// A [`Value`] rather than a typed schema, because a schema is a
    /// document in its own vocabulary and this crate has no business
    /// restating it.
    Schema(Value),
    /// A failure. Tag `1`.
    ///
    /// The image has no schema to give, or the container could not be
    /// asked. See [`shared::error::Error`](crate::shared::error::Error)
    /// for why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Schema`]. Public, so a relay that carries the
/// schema's JSON without reading it can frame it.
pub const SCHEMA: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 1;

impl Encode for Frame {
    /// The ordinary JSON failure, from either half.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Schema(schema) => {
                out.extend_from_slice(&[SCHEMA]);
                serde_json::to_writer(out, schema)
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
            SCHEMA => serde_json::from_slice(rest)
                .map(Frame::Schema)
                .map_err(FrameError::Schema),
            ERROR => {
                Error::decode(rest).map(Frame::Error).map_err(FrameError::Error)
            }
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A schema answer that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is neither of this frame's two.
    UnknownTag(u8),
    /// The schema did not parse.
    Schema(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("schema answer frame is empty"),
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown schema answer frame tag {tag}")
            }
            FrameError::Schema(error) => {
                write!(f, "schema did not parse: {error}")
            }
            FrameError::Error(error) => {
                write!(f, "schema error did not parse: {error}")
            }
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Schema(error) | FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
