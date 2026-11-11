//! What the daemon answers a frontend request with.

use std::fmt;

use bytes::Bytes;
use serde::Serialize;

use crate::shared::containers::{exec, frontend};
use crate::shared::error::Error;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// One answer: a frame of the program's answer, as
/// [`frontend`](crate::shared::containers::frontend) states it —
/// exactly one `Head`, then any number of `Body` — or exactly one
/// `NotFound`, `Forbidden`, `NoFrontend` or `Error`, and the finish; an
/// `Error` also ends a body broken off. As JSON, the bytes of a body
/// are standard base64.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Frame {
    /// Tag `0`: the response's head. First, and once.
    Head(frontend::output::Head),
    /// Tag `1`: a piece of the response's body.
    Body(
        #[serde(serialize_with = "exec::response::base64")]
        #[schemars(with = "String")]
        Bytes,
    ),
    /// Tag `2`: nothing is a dependency tool by that reference — or, for a
    /// dependency tool, none runs.
    NotFound,
    /// Tag `3`: the account holds no `frontend` grant reaching it.
    Forbidden,
    /// Tag `4`: the tool's image declares no frontend.
    NoFrontend,
    /// Tag `5`: no answer, or the daemon failed: the container's words,
    /// or its own.
    Error(Error),
}

const HEAD: u8 = 0;
const BODY: u8 = 1;
const NOT_FOUND: u8 = 2;
const FORBIDDEN: u8 = 3;
const NO_FRONTEND: u8 = 4;
const ERROR: u8 = 5;

impl From<frontend::output::Owned> for Frame {
    /// The container's frame, as the daemon answers it: its `Error`
    /// the daemon's `Error`.
    fn from(owned: frontend::output::Owned) -> Self {
        match owned {
            frontend::output::Owned::Head(head) => Frame::Head(head),
            frontend::output::Owned::Body(bytes) => Frame::Body(bytes),
            frontend::output::Owned::Error(error) => Frame::Error(error),
        }
    }
}

impl Encode for Frame {
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
            Frame::NotFound => {
                out.extend_from_slice(&[NOT_FOUND]);
                Ok(())
            }
            Frame::Forbidden => {
                out.extend_from_slice(&[FORBIDDEN]);
                Ok(())
            }
            Frame::NoFrontend => {
                out.extend_from_slice(&[NO_FRONTEND]);
                Ok(())
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out)
            }
        }
    }
}

/// One answer out of the whole message it arrived in, `bytes`, and its
/// payload: the bytes of a body a view of the message, never copied.
pub fn decode(bytes: &Bytes, payload: &[u8]) -> Result<Frame, FrameError> {
    let (tag, rest) = payload.split_first().ok_or(FrameError::Empty)?;
    Ok(match *tag {
        HEAD => Frame::Head(serde_json::from_slice(rest).map_err(FrameError::Body)?),
        BODY => Frame::Body(bytes.slice_ref(rest)),
        NOT_FOUND => Frame::NotFound,
        FORBIDDEN => Frame::Forbidden,
        NO_FRONTEND => Frame::NoFrontend,
        ERROR => Frame::Error(Error::decode(rest).map_err(FrameError::Body)?),
        tag => return Err(FrameError::UnknownTag(tag)),
    })
}

/// An answer that did not decode.
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
            FrameError::Empty => f.write_str("dependency tools frontend response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown dependency tools frontend response frame tag {tag}"),
            FrameError::Body(error) => write!(f, "dependency tools frontend response did not parse: {error}"),
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
