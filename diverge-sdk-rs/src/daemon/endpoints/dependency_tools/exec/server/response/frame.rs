//! What the daemon answers an exec with.

use std::fmt;

use bytes::Bytes;
use serde::Serialize;

use crate::shared::containers::exec;
use crate::shared::error::Error;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// One answer: a frame of the command's stream, as
/// [`exec`](crate::shared::containers::exec) states it — any number of
/// `Stdout` and `Stderr`, then exactly one `Exited` or `Signaled` — or
/// exactly one `NotFound`, `Forbidden` or `Error`, and the finish. As
/// JSON, the bytes of stdout and stderr are standard base64.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Frame {
    /// Tag `0`: one read of the command's stdout.
    Stdout(
        #[serde(serialize_with = "exec::response::base64")]
        #[schemars(with = "String")]
        Bytes,
    ),
    /// Tag `1`: one read of its stderr.
    Stderr(
        #[serde(serialize_with = "exec::response::base64")]
        #[schemars(with = "String")]
        Bytes,
    ),
    /// Tag `2`: it exited, with this code. Last.
    Exited(i32),
    /// Tag `3`: a signal ended it, this one. Last.
    Signaled(i32),
    /// Tag `4`: nothing is a dependency tool by that reference — or, for a
    /// dependency tool, none runs.
    NotFound,
    /// Tag `5`: the account holds no `exec` grant reaching it.
    Forbidden,
    /// Tag `6`: the command never ran, or the daemon failed: the
    /// container's words, or its own.
    Error(Error),
}

const STDOUT: u8 = 0;
const STDERR: u8 = 1;
const EXITED: u8 = 2;
const SIGNALED: u8 = 3;
const NOT_FOUND: u8 = 4;
const FORBIDDEN: u8 = 5;
const ERROR: u8 = 6;

impl From<exec::response::Owned> for Frame {
    /// The container's frame, as the daemon answers it: its `Error`
    /// the daemon's `Error`.
    fn from(owned: exec::response::Owned) -> Self {
        match owned {
            exec::response::Owned::Stdout(bytes) => Frame::Stdout(bytes),
            exec::response::Owned::Stderr(bytes) => Frame::Stderr(bytes),
            exec::response::Owned::Exited(code) => Frame::Exited(code),
            exec::response::Owned::Signaled(signal) => Frame::Signaled(signal),
            exec::response::Owned::Error(error) => Frame::Error(error),
        }
    }
}

impl Encode for Frame {
    /// The JSON failure of an exit, a signal or an error.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: a variant is called `Error`.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Stdout(bytes) => {
                out.extend_from_slice(&[STDOUT]);
                out.extend_from_slice(bytes);
                Ok(())
            }
            Frame::Stderr(bytes) => {
                out.extend_from_slice(&[STDERR]);
                out.extend_from_slice(bytes);
                Ok(())
            }
            Frame::Exited(code) => {
                out.extend_from_slice(&[EXITED]);
                serde_json::to_writer(out, code)
            }
            Frame::Signaled(signal) => {
                out.extend_from_slice(&[SIGNALED]);
                serde_json::to_writer(out, signal)
            }
            Frame::NotFound => {
                out.extend_from_slice(&[NOT_FOUND]);
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

/// One answer out of the whole message it arrived in, `bytes`, and its
/// payload: the bytes of stdout and stderr a view of the message, never
/// copied.
pub fn decode(bytes: &Bytes, payload: &[u8]) -> Result<Frame, FrameError> {
    let (tag, rest) = payload.split_first().ok_or(FrameError::Empty)?;
    Ok(match *tag {
        STDOUT => Frame::Stdout(bytes.slice_ref(rest)),
        STDERR => Frame::Stderr(bytes.slice_ref(rest)),
        EXITED => Frame::Exited(serde_json::from_slice(rest).map_err(FrameError::Body)?),
        SIGNALED => Frame::Signaled(serde_json::from_slice(rest).map_err(FrameError::Body)?),
        NOT_FOUND => Frame::NotFound,
        FORBIDDEN => Frame::Forbidden,
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
    /// An exit, a signal or an error whose JSON did not parse.
    Body(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("dependency tools exec response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown dependency tools exec response frame tag {tag}"),
            FrameError::Body(error) => write!(f, "dependency tools exec response did not parse: {error}"),
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
