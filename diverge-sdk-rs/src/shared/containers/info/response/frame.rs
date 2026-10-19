//! What an info channel answers.

use std::fmt;

use super::super::Document;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// The image's document, that it has none, or why it could not be
/// read.
///
/// One frame is all there is — this is not a stream, and a channel
/// carrying one of these finishes immediately after.
///
/// # The layout
///
/// ```text
/// [0][document JSON …]   the image's /DIVERGE.md, parsed: its schema and its body
/// [1]                    the image ships no /DIVERGE.md
/// [2][error JSON …]      the file could not be read, or is not a document, and why
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The image's `/DIVERGE.md`, parsed: see [`Document`]. Tag `0`.
    ///
    /// Its `schema` is the JSON Schema of the
    /// [`arguments`](crate::shared::containers::request::Container::arguments)
    /// the image's program accepts, as the image states it; its
    /// `body` is the prose beneath.
    Info(Document),
    /// The image ships no `/DIVERGE.md`. Tag `1`.
    ///
    /// Bare: there is nothing to carry. This is not a failure of
    /// anything but a fact about the image — the proxy looked at the
    /// root and the file was not there — and a caller that asked
    /// learns that the image does not say what it takes.
    Missing,
    /// A failure. Tag `2`.
    ///
    /// The file is there and could not be read, or is there and is
    /// not a document — it breaks a rule of
    /// [`Document::parse`](crate::shared::containers::info::Document::parse)
    /// — or the container could not be asked. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why
    /// it says so little.
    Error(Error),
}

/// Tag for [`Frame::Info`].
const INFO: u8 = 0;

/// Tag for [`Frame::Missing`].
const MISSING: u8 = 1;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 2;

impl Encode for Frame {
    /// The ordinary JSON failure, from either half that has one.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Info(document) => {
                out.extend_from_slice(&[INFO]);
                serde_json::to_writer(out, document)
            }
            Frame::Missing => {
                out.extend_from_slice(&[MISSING]);
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
    /// Four ways to fail, and two of them are JSON.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            INFO => serde_json::from_slice(rest).map(Frame::Info).map_err(FrameError::Info),
            MISSING => Ok(Frame::Missing),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An info answer that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's three.
    UnknownTag(u8),
    /// The document did not parse. The message is the parser's, which
    /// quotes no value.
    Info(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("info answer frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown info answer frame tag {tag}"),
            FrameError::Info(error) => write!(f, "info document did not parse: {error}"),
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
