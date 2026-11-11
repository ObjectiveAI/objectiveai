//! What comes back from a frontend request.

use std::fmt;

use bytes::Bytes;
use serde::{Deserialize, Serialize};

use crate::shared::error::Error;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

const HEAD: u8 = 0;
const BODY: u8 = 1;
const ERROR: u8 = 2;

/// An HTTP response's head, as JSON: its status and its headers in
/// their order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Head {
    /// The status code.
    pub status: u16,
    /// The headers, each a name and a value, in their order; a name may
    /// repeat.
    #[serde(default)]
    pub headers: Vec<(String, String)>,
}

/// One frame of a frontend request's output: one byte of tag, then
/// JSON for `Head` and `Error`, the bytes as they are for `Body` (see
/// [`frontend`](super)).
#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    /// Tag `0`: the response's head. First, and once.
    Head(Head),
    /// Tag `1`: a piece of the response's body, or, after a `101`, of
    /// what the program sends.
    Body(&'a [u8]),
    /// Tag `2`: no response, and why; or, after a head, the body broken
    /// off. Last.
    Error(Error),
}

impl Frame<'_> {
    /// The frame as an [`Owned`], its bytes a view of `whole`, the
    /// message it was decoded out of.
    pub fn owned(&self, whole: &Bytes) -> Owned {
        match self {
            Frame::Head(head) => Owned::Head(head.clone()),
            Frame::Body(bytes) => Owned::Body(whole.slice_ref(bytes)),
            Frame::Error(error) => Owned::Error(error.clone()),
        }
    }
}

/// A [`Frame`] that owns its bytes.
#[derive(Debug, Clone, PartialEq)]
pub enum Owned {
    /// See [`Frame::Head`].
    Head(Head),
    /// See [`Frame::Body`].
    Body(Bytes),
    /// See [`Frame::Error`].
    Error(Error),
}

impl Owned {
    /// The borrowed frame, for encoding again.
    pub fn as_frame(&self) -> Frame<'_> {
        match self {
            Owned::Head(head) => Frame::Head(head.clone()),
            Owned::Body(bytes) => Frame::Body(bytes),
            Owned::Error(error) => Frame::Error(error.clone()),
        }
    }
}

impl Encode for Frame<'_> {
    /// The JSON failure of a head or an error.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: a variant is called `Error`.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Head(head) => {
                out.extend_from_slice(&[HEAD]);
                serde_json::to_writer(out, head)
            }
            Frame::Body(bytes) => {
                out.extend_from_slice(&[BODY]);
                out.extend_from_slice(bytes);
                Ok(())
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                serde_json::to_writer(out, error)
            }
        }
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// See [`FrameError`].
    type Error = FrameError;

    fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (&tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        Ok(match tag {
            HEAD => Frame::Head(serde_json::from_slice(rest).map_err(FrameError::Body)?),
            BODY => Frame::Body(rest),
            ERROR => Frame::Error(serde_json::from_slice(rest).map_err(FrameError::Body)?),
            other => return Err(FrameError::UnknownTag(other)),
        })
    }
}

/// A frame of a frontend request's output that did not decode.
#[derive(Debug)]
pub enum FrameError {
    /// No tag at all.
    Empty,
    /// A tag no variant is.
    UnknownTag(u8),
    /// A head or an error whose JSON did not parse.
    Body(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("frontend output frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown frontend output frame tag {tag}"),
            FrameError::Body(error) => write!(f, "frontend output frame did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Body(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
