//! What a server's channel request frame carries for a FUSE mount.

use std::fmt;

use super::{Path, Rename, Write};
use crate::decode::Decode;
use crate::encode::{Encode, Writer};

/// What a mount asks the server for, on behalf of the program using
/// it.
///
/// A payload leads with one byte saying which, and the rest is that
/// variant's own bytes. No ask names the mount: the scope it rides is
/// the mount.
///
/// | tag | asks for | payload |
/// |-----|----------|---------|
/// | `0` | [`Read`](Self::Read) | `[path…]` |
/// | `1` | [`Write`](Self::Write) | `[path_len: u16 BE][path…][bytes…]` |
/// | `2` | [`List`](Self::List) | `[path…]` |
/// | `3` | [`Remove`](Self::Remove) | `[path…]` |
/// | `4` | [`Rename`](Self::Rename) | `[from_len: u16 BE][from…][to…]` |
/// | `5` | [`Mkdir`](Self::Mkdir) | `[path…]` |
/// | `6` | [`Stat`](Self::Stat) | `[path…]` |
///
/// Where a path is the last thing in a payload it runs to the end;
/// where bytes or a second path follow it, it carries a length
/// prefix. The seven are the seven of
/// [`shared::containers::fuse`](crate::shared::containers::fuse), and
/// each is answered with that module's own frame, one message and
/// the finish; what each MEANS — a file mount's empty path, the root
/// that is never removed or moved, whose refusal a refusal is — is
/// stated there once.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame<'a> {
    /// Read a file of the mount, whole. Tag `0`.
    ///
    /// Every open of the file. See
    /// [`read`](crate::shared::containers::fuse::read).
    Read(Path<'a>),
    /// Write a file of the mount, whole. Tag `1`.
    ///
    /// Every changed close of the file. See
    /// [`write`](crate::shared::containers::fuse::write).
    Write(Write<'a>),
    /// List a directory of the mount. Tag `2`.
    ///
    /// Every listing in a directory mount: names and kinds. See
    /// [`list`](crate::shared::containers::fuse::list).
    List(Path<'a>),
    /// Remove a file or an empty directory of the mount. Tag `3`.
    ///
    /// See [`remove`](crate::shared::containers::fuse::remove).
    Remove(Path<'a>),
    /// Rename an entry within the mount. Tag `4`.
    ///
    /// See [`rename`](crate::shared::containers::fuse::rename).
    Rename(Rename<'a>),
    /// Make a directory in the mount. Tag `5`.
    ///
    /// See [`mkdir`](crate::shared::containers::fuse::mkdir).
    Mkdir(Path<'a>),
    /// What an entry of the mount is, and how long. Tag `6`.
    ///
    /// Every attribute of a file mount, and every lookup and
    /// attribute of an entry in a directory mount: a `stat` costs
    /// nine bytes back, not the file. See
    /// [`stat`](crate::shared::containers::fuse::stat).
    Stat(Path<'a>),
}

/// Tag for [`Frame::Read`].
const READ: u8 = 0;

/// Tag for [`Frame::Write`].
const WRITE: u8 = 1;

/// Tag for [`Frame::List`].
const LIST: u8 = 2;

/// Tag for [`Frame::Remove`].
const REMOVE: u8 = 3;

/// Tag for [`Frame::Rename`].
const RENAME: u8 = 4;

/// Tag for [`Frame::Mkdir`].
const MKDIR: u8 = 5;

/// Tag for [`Frame::Stat`].
const STAT: u8 = 6;

impl Encode for Frame<'_> {
    /// One way to fail: a path too long for its prefix, on the two
    /// asks where a path has one. Everything else is bytes copied.
    type Error = FrameEncodeError;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), FrameEncodeError> {
        match self {
            Frame::Read(path) => {
                out.extend_from_slice(&[READ]);
                // Its error is `Infallible`, and an empty match on one
                // is how you say so: there is no value to handle.
                path.encode(out).map_err(|error| match error {})
            }
            Frame::Write(write) => {
                out.extend_from_slice(&[WRITE]);
                write.encode(out)
            }
            Frame::List(path) => {
                out.extend_from_slice(&[LIST]);
                path.encode(out).map_err(|error| match error {})
            }
            Frame::Remove(path) => {
                out.extend_from_slice(&[REMOVE]);
                path.encode(out).map_err(|error| match error {})
            }
            Frame::Rename(rename) => {
                out.extend_from_slice(&[RENAME]);
                rename.encode(out)
            }
            Frame::Mkdir(path) => {
                out.extend_from_slice(&[MKDIR]);
                path.encode(out).map_err(|error| match error {})
            }
            Frame::Stat(path) => {
                out.extend_from_slice(&[STAT]);
                path.encode(out).map_err(|error| match error {})
            }
        }
    }
}

/// A mount ask that could not be written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameEncodeError {
    /// A path of more bytes than a two-byte length prefix can say,
    /// carrying how many there were. Only the asks where something
    /// follows the path prefix it; a path that ends the payload takes
    /// any length.
    PathLength(usize),
}

impl fmt::Display for FrameEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameEncodeError::PathLength(len) => {
                write!(f, "fuse path is {len} bytes, more than 65535")
            }
        }
    }
}

impl std::error::Error for FrameEncodeError {}

impl<'a> Decode<'a> for Frame<'a> {
    /// Four ways to fail, and none of them is a parse.
    type Error = FrameError;

    fn decode(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            READ => Path::decode(rest).map(Frame::Read),
            WRITE => Write::decode(rest).map(Frame::Write),
            LIST => Path::decode(rest).map(Frame::List),
            REMOVE => Path::decode(rest).map(Frame::Remove),
            RENAME => Rename::decode(rest).map(Frame::Rename),
            MKDIR => Path::decode(rest).map(Frame::Mkdir),
            STAT => Path::decode(rest).map(Frame::Stat),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A mount ask that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's seven.
    UnknownTag(u8),
    /// Fewer bytes than the ask's fixed part promises — a length
    /// prefix, or the path a prefix said was there.
    Truncated,
    /// A path that is not UTF-8.
    PathUtf8,
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("fuse mount channel request frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown fuse mount channel request tag {tag}")
            }
            FrameError::Truncated => {
                f.write_str("fuse ask is shorter than it promises")
            }
            FrameError::PathUtf8 => f.write_str("fuse path is not utf-8"),
        }
    }
}

impl std::error::Error for FrameError {}
