//! What a caller answers on a registration channel.

use std::fmt;

use super::super::super::secrets::Values;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// Deployed, or not.
///
/// The whole answer to a [`Request`](super::super::request::Request),
/// and one frame is all there is — this is not a stream, and a
/// channel carrying one of these finishes immediately after.
///
/// # The layout
///
/// ```text
/// [0][values JSON …]   deployed: every dependency runs, and these are the secrets' first values
/// [1][error JSON …]    not deployed, and why
/// ```
///
/// A deploy carries the first [`Values`] — every name the program
/// declared, each to the value locked or read for it — which the
/// provider feeds to the container before it sends the id, and
/// nothing else: the provider needs no id of a tool container, since
/// the caller wires them. An empty map is a program that declared no
/// secret. A refusal carries the caller's own words — a dependency
/// that did not deploy, a name the template does not declare, a key
/// with no free value — which the run's error carries on, and never
/// a value.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// Every dependency runs, and the secrets have these values. Tag
    /// `0`.
    Deployed(Values),
    /// Something did not, and this says why. Tag `1`. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why
    /// it says so little.
    Error(Error),
}

/// Tag for [`Frame::Deployed`].
const DEPLOYED: u8 = 0;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 1;

impl Encode for Frame {
    /// The ordinary JSON failure, from either variant.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Deployed(values) => {
                out.extend_from_slice(&[DEPLOYED]);
                serde_json::to_writer(out, values)
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
            DEPLOYED => serde_json::from_slice(rest).map(Frame::Deployed).map_err(FrameError::Deployed),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A registration answer that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is neither of this frame's two.
    UnknownTag(u8),
    /// The values did not parse. The message is the parser's, which
    /// quotes no value.
    Deployed(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("registration answer frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown registration answer frame tag {tag}"),
            FrameError::Deployed(error) => write!(f, "registration values did not parse: {error}"),
            FrameError::Error(error) => write!(f, "registration error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Deployed(error) | FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
