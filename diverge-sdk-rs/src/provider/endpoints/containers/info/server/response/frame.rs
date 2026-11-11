//! What a provider's response frame carries for an image's info.

use std::fmt;

use crate::shared::containers::info::Document;
use crate::shared::error::Error;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// The image's document, that it has none, that the image was found
/// nowhere, or why it could not be read.
///
/// Exactly one is sent, and the finish follows it.
///
/// # The layout
///
/// ```text
/// [0][document JSON …]   the image's /DIVERGE.md, parsed: its name, its schema and its body
/// [1]                    the image ships no /DIVERGE.md
/// [2]                    the image is in no store and no registry the provider may ask
/// [3][error JSON …]      the file is not a document, or the image could not be read, and why
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The image's `/DIVERGE.md`, parsed: see [`Document`]. Tag `0`.
    Info(Document),
    /// The image ships no `/DIVERGE.md`, or what is at that path is
    /// not a regular file. Tag `1`.
    ///
    /// A fact about the image and not a failure: the provider read the
    /// image's root and the file was not there.
    Missing,
    /// The provider found the image in none of the places a run would
    /// look — the caller's own images, what it holds, the referenced
    /// registries its policy lists — and read nothing. Tag `2`.
    NoSource,
    /// A failure. Tag `3`.
    ///
    /// The file is there and is not a document — it breaks a rule of
    /// [`Document::parse`](crate::shared::containers::info::Document::parse),
    /// told as kind `info` — or the image could not be obtained or
    /// read. See [`shared::error::Error`](crate::shared::error::Error)
    /// for why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Info`].
const INFO: u8 = 0;

/// Tag for [`Frame::Missing`].
const MISSING: u8 = 1;

/// Tag for [`Frame::NoSource`].
const NO_SOURCE: u8 = 2;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 3;

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
            Frame::NoSource => {
                out.extend_from_slice(&[NO_SOURCE]);
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
            NO_SOURCE => Ok(Frame::NoSource),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An image info answer that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's four.
    UnknownTag(u8),
    /// The document did not parse.
    Info(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("image info answer frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown image info answer frame tag {tag}"),
            FrameError::Info(error) => write!(f, "image info document did not parse: {error}"),
            FrameError::Error(error) => write!(f, "image info error did not parse: {error}"),
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
