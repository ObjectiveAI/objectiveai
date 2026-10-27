//! What a server's response frame carries for an image's info.

use serde::Serialize;
use std::fmt;

use crate::shared::containers::info::Document;
use crate::shared::error::Error;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// An image info's answer: the document, that the image ships none,
/// that nothing could be asked, forbidden, or a failure.
///
/// One question and one reply, so there is exactly one of these per
/// scope, before the finish that ends it. A payload leads with one byte
/// saying which — `0` for [`Info`](Self::Info), `1` for
/// [`Missing`](Self::Missing), `2` for [`NoSource`](Self::NoSource), `3`
/// for [`Forbidden`](Self::Forbidden), `4` for [`Error`](Self::Error) —
/// and the rest is that variant's own JSON: the document for the first,
/// nothing for the bare answers, the error for the last.
///
/// # Answers, and one failure
///
/// [`Info`](Self::Info), [`Missing`](Self::Missing) and
/// [`NoSource`](Self::NoSource) are ANSWERS. An [`Error`](Self::Error)
/// is the absence of one: the container or the provider asked could
/// not say, in whichever's words. Nothing is retried, and no second
/// container or provider is asked after a failure.
///
/// # Forbidden
///
/// [`Forbidden`](Self::Forbidden) is an answer every endpoint has: the
/// account the request is served for — the connection's, or the
/// `account` of the container it came from — holds no grant allowing
/// what the request asks, and nothing was asked. See
/// [`grant`](crate::daemon::grant).
///
/// # As JSON
///
/// For a client that prints what the daemon answered: the variant,
/// snake case, over its payload — `{"info":…}`, `{"error":…}` — and a
/// bare variant the string alone — `"missing"`. The wire is the tag
/// byte and the payload, never this.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Frame {
    /// The image's `/DIVERGE.md`, parsed: see [`Document`]. Tag `0`.
    Info(Document),
    /// The image ships no `/DIVERGE.md`. Tag `1`.
    Missing,
    /// No container of the image runs, and no connected provider can
    /// obtain the image: nothing was asked. Tag `2`.
    NoSource,
    /// The account the request is served for holds no grant allowing
    /// it; nothing was asked. Tag `3`.
    Forbidden,
    /// A failure. Tag `4`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Info`].
const INFO: u8 = 0;

/// Tag for [`Frame::Missing`].
const MISSING: u8 = 1;

/// Tag for [`Frame::NoSource`].
const NO_SOURCE: u8 = 2;

/// Tag for [`Frame::Forbidden`].
const FORBIDDEN: u8 = 3;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 4;

/// A tag, then the variant's own JSON, if it has any.
impl Encode for Frame {
    /// The ordinary JSON failure. The bare answers cannot fail.
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
            MISSING => Ok(Frame::Missing),
            NO_SOURCE => Ok(Frame::NoSource),
            FORBIDDEN => Ok(Frame::Forbidden),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An images info response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's five.
    UnknownTag(u8),
    /// The document did not parse.
    Info(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("images info response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown images info response frame tag {tag}"),
            FrameError::Info(error) => write!(f, "images info document did not parse: {error}"),
            FrameError::Error(error) => write!(f, "images info error did not parse: {error}"),
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
