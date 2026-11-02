//! What comes out of a terminal.

use std::fmt;

use bytes::Bytes;

use crate::shared::error::Error;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

const STDOUT: u8 = 0;
const EXITED: u8 = 1;
const SIGNALED: u8 = 2;
const ERROR: u8 = 3;

/// One frame of a terminal's output: one byte of tag, then the bytes
/// as they are for `Stdout`, JSON for the rest (see
/// [`terminal`](super)).
#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    /// Tag `0`: one read of the terminal — stdout and stderr together.
    Stdout(&'a [u8]),
    /// Tag `1`: the shell exited, with this code. Last.
    Exited(i32),
    /// Tag `2`: a signal ended it, this one. Last.
    Signaled(i32),
    /// Tag `3`: no shell started, and why. The only frame.
    Error(Error),
}

impl Frame<'_> {
    /// The frame as an [`Owned`], its bytes a view of `whole`, the
    /// message it was decoded out of.
    pub fn owned(&self, whole: &Bytes) -> Owned {
        match self {
            Frame::Stdout(bytes) => Owned::Stdout(whole.slice_ref(bytes)),
            Frame::Exited(code) => Owned::Exited(*code),
            Frame::Signaled(signal) => Owned::Signaled(*signal),
            Frame::Error(error) => Owned::Error(error.clone()),
        }
    }
}

/// A [`Frame`] that owns its bytes.
#[derive(Debug, Clone, PartialEq)]
pub enum Owned {
    /// See [`Frame::Stdout`].
    Stdout(Bytes),
    /// See [`Frame::Exited`].
    Exited(i32),
    /// See [`Frame::Signaled`].
    Signaled(i32),
    /// See [`Frame::Error`].
    Error(Error),
}

impl Owned {
    /// The borrowed frame, for encoding again.
    pub fn as_frame(&self) -> Frame<'_> {
        match self {
            Owned::Stdout(bytes) => Frame::Stdout(bytes),
            Owned::Exited(code) => Frame::Exited(*code),
            Owned::Signaled(signal) => Frame::Signaled(*signal),
            Owned::Error(error) => Frame::Error(error.clone()),
        }
    }
}

impl Encode for Frame<'_> {
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
            STDOUT => Frame::Stdout(rest),
            EXITED => Frame::Exited(serde_json::from_slice(rest).map_err(FrameError::Body)?),
            SIGNALED => Frame::Signaled(serde_json::from_slice(rest).map_err(FrameError::Body)?),
            ERROR => Frame::Error(serde_json::from_slice(rest).map_err(FrameError::Body)?),
            other => return Err(FrameError::UnknownTag(other)),
        })
    }
}

/// A frame of a terminal's output that did not decode.
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
            FrameError::Empty => f.write_str("terminal output frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown terminal output frame tag {tag}"),
            FrameError::Body(error) => write!(f, "terminal output frame did not parse: {error}"),
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
