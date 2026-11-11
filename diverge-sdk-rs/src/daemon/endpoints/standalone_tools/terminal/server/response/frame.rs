//! What the daemon answers a terminal with.

use std::fmt;

use bytes::Bytes;
use serde::Serialize;

use crate::shared::containers::{exec, terminal};
use crate::shared::error::Error;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// One answer: a frame of the terminal's output, as
/// [`terminal`](crate::shared::containers::terminal) states it — any
/// number of `Stdout`, then exactly one `Exited` or `Signaled` — or
/// exactly one `NotFound`, `Forbidden` or `Error`, and the finish. As
/// JSON, the bytes of stdout are standard base64.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Frame {
    /// Tag `0`: one read of the terminal — stdout and stderr together.
    Stdout(
        #[serde(serialize_with = "exec::response::base64")]
        #[schemars(with = "String")]
        Bytes,
    ),
    /// Tag `1`: the shell exited, with this code. Last.
    Exited(i32),
    /// Tag `2`: a signal ended it, this one. Last.
    Signaled(i32),
    /// Tag `3`: nothing is a standalone tool by that reference — or, for a
    /// dependency tool, none runs.
    NotFound,
    /// Tag `4`: the account holds no `exec` grant reaching it.
    Forbidden,
    /// Tag `5`: no shell started, or the daemon failed: the
    /// container's words, or its own.
    Error(Error),
}

const STDOUT: u8 = 0;
const EXITED: u8 = 1;
const SIGNALED: u8 = 2;
const NOT_FOUND: u8 = 3;
const FORBIDDEN: u8 = 4;
const ERROR: u8 = 5;

impl From<terminal::output::Owned> for Frame {
    /// The container's frame, as the daemon answers it: its `Error`
    /// the daemon's `Error`.
    fn from(owned: terminal::output::Owned) -> Self {
        match owned {
            terminal::output::Owned::Stdout(bytes) => Frame::Stdout(bytes),
            terminal::output::Owned::Exited(code) => Frame::Exited(code),
            terminal::output::Owned::Signaled(signal) => Frame::Signaled(signal),
            terminal::output::Owned::Error(error) => Frame::Error(error),
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
/// payload: the bytes of stdout a view of the message, never copied.
pub fn decode(bytes: &Bytes, payload: &[u8]) -> Result<Frame, FrameError> {
    let (tag, rest) = payload.split_first().ok_or(FrameError::Empty)?;
    Ok(match *tag {
        STDOUT => Frame::Stdout(bytes.slice_ref(rest)),
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
            FrameError::Empty => f.write_str("standalone tools terminal response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown standalone tools terminal response frame tag {tag}"),
            FrameError::Body(error) => write!(f, "standalone tools terminal response did not parse: {error}"),
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
