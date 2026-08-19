//! What a client's channel request frame carries for a watch.

use std::error::Error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// Stop watching.
///
/// The only thing a caller asks of a running watch, and the whole of
/// what this channel carries. A payload is one byte and nothing else.
///
/// # Why a watch has one and the other volume scopes do not
///
/// Because a watch is the only one that does not end by itself. A
/// listing answers and finishes; a creation happens and finishes. A
/// watch reports changes for as long as the scope stays open, so
/// without this the only way to be done with one is to stop reading —
/// which tells the provider nothing, and leaves it watching a tree
/// nobody is listening about.
///
/// # It has no answer, and does not need one
///
/// Nothing comes back on this channel. What comes back is the end of
/// the SCOPE — a
/// [`ResponseFinish`](crate::frame::server::ServerFrame::ResponseFinish),
/// which already means nothing bearing this scope follows on any
/// channel. Finishing this one first would be a smaller way of saying
/// the same thing, moments earlier.
///
/// So a caller sees its watch end the way a watch always ends: the
/// stream stops, with a finish rather than a silence. Which is the
/// distinction worth preserving — a watch that ended is not a watch
/// whose connection went.
///
/// # What it adds over closing the connection
///
/// Leaving ends this scope either way. The difference is that a
/// provider cannot tell a deliberate exit from a network that stopped
/// answering, and has to wait to find out — during which it is still
/// watching, still walking a tree, and still sending frames to a
/// receiver nobody holds. This is unambiguous and immediate.
///
/// # What it does not do
///
/// Touch the volume. A watch is a way of looking at one, and looking
/// away leaves everything where it was — the volume, its contents, and
/// any other caller watching the same tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Frame;

/// The tag that says stop watching.
const DISCONNECT: u8 = 0;

impl Encode for Frame {
    /// [`Infallible`](std::convert::Infallible): one known byte.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        out.extend_from_slice(&[DISCONNECT]);
        Ok(())
    }
}

impl Decode<'_> for Frame {
    /// Two ways to fail, and neither is a parse.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        match bytes.first() {
            Some(&DISCONNECT) => Ok(Frame),
            Some(&byte) => Err(FrameError::UnknownTag(byte)),
            None => Err(FrameError::Empty),
        }
    }
}

/// A watch channel request that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag this version does not define.
    ///
    /// Which is what a caller asking for something else will send,
    /// once there is something else to ask for. Until then it is a
    /// peer that disagrees about the protocol.
    UnknownTag(u8),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("watch channel request frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown watch channel request tag {tag}")
            }
        }
    }
}

impl Error for FrameError {}
