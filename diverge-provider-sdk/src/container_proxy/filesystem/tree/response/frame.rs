//! One event of the tree, or why there will be no more.

use super::FrameError;
use crate::decode::Decode as _;
use crate::encode::{Encode, Writer};
use crate::shared::filetree;

/// One message on `/filesystem/tree`.
///
/// ```text
/// [kind: u8][postcard frame… | message…]
/// ```
///
/// The first on a connection is a
/// [`Snapshot`](filetree::response::Frame::Snapshot) behind kind `0`;
/// every one after is a delta, or a snapshot again when the watch
/// lost events; and an [`Error`](Self::Error) is the last, when the
/// watch could not exist at all.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    /// Kind `0`. One filetree event, as
    /// [`shared::filetree`](crate::shared::filetree) encodes it.
    Filetree(filetree::response::Frame),
    /// Kind `1`. The watch could not be made, the root could not be
    /// watched, or the walk died — and this says why, for a reader
    /// rather than a program. The last message before the close;
    /// nothing follows it, and the server starts over when it likes.
    Error(&'a str),
}

impl Encode for Frame<'_> {
    /// postcard's own failure, from the half that has one; a kind
    /// byte and a message's bytes cannot fail.
    type Error = postcard::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), postcard::Error> {
        match self {
            Frame::Filetree(frame) => {
                out.extend_from_slice(&[0]);
                frame.encode(out)
            }
            Frame::Error(message) => {
                out.extend_from_slice(&[1]);
                out.extend_from_slice(message.as_bytes());
                Ok(())
            }
        }
    }
}

impl<'a> Frame<'a> {
    /// Decode one message. The error message borrows from `bytes`.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, FrameError> {
        let (kind, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *kind {
            0 => filetree::response::Frame::decode(rest)
                .map(Frame::Filetree)
                .map_err(FrameError::Filetree),
            1 => std::str::from_utf8(rest)
                .map(Frame::Error)
                .map_err(|_| FrameError::MessageUtf8),
            other => Err(FrameError::UnknownKind(other)),
        }
    }
}
