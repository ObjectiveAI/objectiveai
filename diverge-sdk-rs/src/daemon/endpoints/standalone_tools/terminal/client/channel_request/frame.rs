//! The channels a client opens on a terminal.
//!
//! The rules:
//!
//! 1. `Close` kills the shell; the daemon answers nothing on the
//!    channel and finishes the scope.
//! 2. `Stdin` and `Resize` are each answered by the channel's finish,
//!    and nothing before it, once done. A client sends its next `Stdin`
//!    once the last is finished, which is what keeps them in order.
//! 3. A channel opened after the shell ended is answered by its finish
//!    and does nothing.

use std::fmt;

use crate::shared::containers::terminal::open::Open;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// One channel on a terminal: one byte of tag, then nothing for
/// `Close`, the bytes as they are for `Stdin`, JSON for `Resize`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame<'a> {
    /// Tag `0`: kill the shell.
    Close,
    /// Tag `1`: write these bytes to the shell's stdin.
    Stdin(&'a [u8]),
    /// Tag `2`: set the terminal's size.
    Resize(Open),
}

const CLOSE: u8 = 0;
const STDIN: u8 = 1;
const RESIZE: u8 = 2;

impl Encode for Frame<'_> {
    /// The JSON failure of a size.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Close => {
                out.extend_from_slice(&[CLOSE]);
                Ok(())
            }
            Frame::Stdin(bytes) => {
                out.extend_from_slice(&[STDIN]);
                out.extend_from_slice(bytes);
                Ok(())
            }
            Frame::Resize(size) => {
                out.extend_from_slice(&[RESIZE]);
                serde_json::to_writer(out, size)
            }
        }
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// See [`FrameError`].
    type Error = FrameError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            CLOSE => Ok(Frame::Close),
            STDIN => Ok(Frame::Stdin(rest)),
            RESIZE => serde_json::from_slice(rest).map(Frame::Resize).map_err(FrameError::Body),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A channel request that did not decode.
#[derive(Debug)]
pub enum FrameError {
    /// No tag at all.
    Empty,
    /// A tag no variant is.
    UnknownTag(u8),
    /// A size whose JSON did not parse.
    Body(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("standalone tools terminal channel request frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown standalone tools terminal channel request tag {tag}"),
            FrameError::Body(error) => write!(f, "standalone tools terminal resize did not parse: {error}"),
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
