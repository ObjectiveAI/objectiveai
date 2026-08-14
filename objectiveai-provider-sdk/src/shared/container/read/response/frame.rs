//! What a response frame carries on a read channel.

use std::error::Error;
use std::fmt;

use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// The file's bytes, and whether to believe them.
///
/// # Two variants, because the third is already on the wire
///
/// There is no `Complete`. A read that finished cleanly is one whose
/// channel finished — [`ChannelResponseFinish`](crate::frame::server::ServerFrame::ChannelResponseFinish)
/// says so at the frame layer, and saying it again in the payload
/// would be two signals for one fact.
///
/// Which leaves three outcomes a caller can tell apart:
///
/// | the channel ends with | means |
/// |-----------------------|-------|
/// | bodies, then a finish | the bytes are the file |
/// | bodies, a [`Corrupted`](Self::Corrupted), then a finish | you have bytes; they may not be a file |
/// | bodies, then nothing | the transfer did not finish |
///
/// # No length, anywhere
///
/// Not in a head, not in the request, not implied by anything. A file
/// being written to can change size in both directions after a sender
/// has looked at it, so any length stated up front is a promise made
/// about a number that has already moved. It is the commitment that
/// makes `tar` corrupt a whole archive when one entry shifts, and this
/// declines to make it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Frame<'a> {
    /// A piece of the file. Tag `0`.
    ///
    /// Sent as it is read. A sender holds no more than one buffer's
    /// worth at a time, whatever the file's size — which is the only
    /// way a thirty-gigabyte file moves through a container with ten
    /// gigabytes free.
    Body(&'a [u8]),
    /// The file changed while it was being read. Tag `1`.
    ///
    /// The bytes already sent are a mix: whatever was there before the
    /// change, up to wherever the reader had got, and whatever was
    /// there after it, from that point on. A file that never existed
    /// in that state at any instant.
    ///
    /// # How a sender knows
    ///
    /// By `fstat` on its own descriptor before the first byte and
    /// after the last, comparing size and modification time. Not
    /// `stat` on the path — a file replaced by a rename leaves the
    /// descriptor reading the original inode coherently to its end,
    /// which is the good case and is not this.
    ///
    /// # And where it cannot
    ///
    /// A same-size overwrite inside one timestamp granule moves
    /// neither field, and is invisible. This is detection, not a
    /// guarantee, and the absence of this frame is not proof of
    /// anything. Where the filesystem supports it,
    /// `statx(STATX_CHANGE_COOKIE)` closes that gap; nothing requires
    /// a provider to have it.
    ///
    /// # What it does not do
    ///
    /// Prevent. Nothing can. Linux advisory locks bind only processes
    /// that ask for them, and mandatory locking was removed in 5.15 —
    /// so an arbitrary process in the container writes straight
    /// through anything a reader might try to hold.
    Corrupted,
}

/// Tag for [`Frame::Body`].
const BODY: u8 = 0;

/// Tag for [`Frame::Corrupted`].
const CORRUPTED: u8 = 1;

impl Encode for Frame<'_> {
    /// [`Infallible`](std::convert::Infallible): a tag and a slice
    /// copy, neither of which can fail.
    type Error = std::convert::Infallible;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::Body(body) => {
                out.extend_from_slice(&[BODY]);
                out.extend_from_slice(body);
            }
            Frame::Corrupted => out.extend_from_slice(&[CORRUPTED]),
        }
        Ok(())
    }
}

impl<'a> Decode<'a> for Frame<'a> {
    /// Two ways to fail, and neither is a parse.
    type Error = FrameError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            BODY => Ok(Frame::Body(rest)),
            CORRUPTED => Ok(Frame::Corrupted),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A read response frame that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    ///
    /// Distinct from a zero-length body, which is a tag followed by
    /// nothing and is ordinary — an empty file is a real file.
    Empty,
    /// A tag that is neither [`Frame::Body`] nor
    /// [`Frame::Corrupted`].
    UnknownTag(u8),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("read response frame is empty"),
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown read response frame tag {tag}")
            }
        }
    }
}

impl Error for FrameError {}
