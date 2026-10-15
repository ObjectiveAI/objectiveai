//! What a server's response frame carries for a volume stat.

use std::fmt;

use super::Stat;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// The volume examined, or the news that it could not be.
///
/// One of these on channel `0`, then the scope finishes. A payload
/// leads with one byte saying which — `0` for [`Stat`](Self::Stat),
/// `1` for [`Mounted`](Self::Mounted), `2` for
/// [`FuseServed`](Self::FuseServed), `3` for [`Held`](Self::Held),
/// `4` for [`Error`](Self::Error) — and the rest is that variant's
/// own bytes, of which the three refusals have none.
///
/// # Mounted, FUSE served and held are answers, not errors
///
/// A volume that something holds is not touched, and the provider
/// says which thing holds it — a running container, a FUSE serve, or
/// another operation — with its own variant rather than an
/// [`Error`](Self::Error) a caller could not tell from any other
/// failure. The distinction is what a caller acts on: stop the
/// container, stop the FUSE serve, or wait, and ask again.
///
///
/// A stat is not a stream: a provider walks the volume once and says
/// what it found, so there is nothing to discover incrementally and
/// nothing to hold a channel open for.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The volume examined. Tag `0`.
    Stat(Stat),
    /// The volume is mounted in a running container, and was not
    /// examined. Tag `1`.
    ///
    /// Nothing was changed; a caller stops the container and asks
    /// again.
    Mounted,
    /// A FUSE serve holds the volume, and it was not examined.
    /// Tag `2`.
    ///
    /// Nothing was changed; a caller stops the FUSE serve and asks
    /// again.
    FuseServed,
    /// Another operation — a stat, a read, a write, a filetree, an
    /// edit or a delete — has the volume to itself, and it was not
    /// examined. Tag `3`.
    ///
    /// Nothing was changed; a caller waits for it to end and asks
    /// again.
    Held,
    /// A failure. Tag `4`.
    ///
    /// A volume the caller cannot see is one, and so is a walk the
    /// provider could not finish. See
    /// [`shared::error::Error`](crate::shared::error::Error) for why
    /// it says so little.
    Error(Error),
}

/// Tag for [`Frame::Stat`].
const STAT: u8 = 0;

/// Tag for [`Frame::Mounted`].
const MOUNTED: u8 = 1;

/// Tag for [`Frame::FuseServed`].
const FUSE_SERVED: u8 = 2;

/// Tag for [`Frame::Held`].
const HELD: u8 = 3;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 4;

/// Two variants, two formats, and the tag chooses between them.
///
/// The stat is **postcard**, matching the rest of
/// [`volumes`](crate::provider::endpoints::volumes): this relays nothing, so no
/// byte of it has to survive a round trip unchanged, and nothing
/// downstream reads it as text.
///
/// The error is JSON, and has to be. A
/// [`serde_json::Value`] deserializes through `deserialize_any`, which
/// a format with no self-description cannot answer — so postcard can
/// carry the stat and cannot carry the failure, and each variant gets
/// the format it needs.
impl Encode for Frame {
    /// One failure per half, and they are different libraries'.
    type Error = FrameEncodeError;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(
        &self,
        out: &mut Writer<'_>,
    ) -> Result<(), FrameEncodeError> {
        match self {
            Frame::Stat(stat) => {
                out.extend_from_slice(&[STAT]);
                postcard::to_io(stat, &mut *out)
                    .map(|_| ())
                    .map_err(FrameEncodeError::Stat)
            }
            Frame::Mounted => {
                out.extend_from_slice(&[MOUNTED]);
                Ok(())
            }
            Frame::FuseServed => {
                out.extend_from_slice(&[FUSE_SERVED]);
                Ok(())
            }
            Frame::Held => {
                out.extend_from_slice(&[HELD]);
                Ok(())
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out).map_err(FrameEncodeError::Error)
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Four ways to fail, and each names which half failed.
    type Error = FrameDecodeError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameDecodeError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameDecodeError::Empty)?;
        match *tag {
            STAT => postcard::from_bytes(rest)
                .map(Frame::Stat)
                .map_err(FrameDecodeError::Stat),
            MOUNTED => Ok(Frame::Mounted),
            FUSE_SERVED => Ok(Frame::FuseServed),
            HELD => Ok(Frame::Held),
            ERROR => Error::decode(rest)
                .map(Frame::Error)
                .map_err(FrameDecodeError::Error),
            tag => Err(FrameDecodeError::UnknownTag(tag)),
        }
    }
}

/// A volume stat that could not be written.
#[derive(Debug)]
pub enum FrameEncodeError {
    /// The stat did not serialize.
    Stat(postcard::Error),
    /// The error did not serialize.
    Error(serde_json::Error),
}

impl fmt::Display for FrameEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameEncodeError::Stat(error) => {
                write!(f, "volume stat did not serialize: {error}")
            }
            FrameEncodeError::Error(error) => {
                write!(f, "volume stat error did not serialize: {error}")
            }
        }
    }
}

impl std::error::Error for FrameEncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameEncodeError::Stat(error) => Some(error),
            FrameEncodeError::Error(error) => Some(error),
        }
    }
}

/// A volume stat that could not be read.
#[derive(Debug)]
pub enum FrameDecodeError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's five.
    UnknownTag(u8),
    /// The stat did not parse.
    Stat(postcard::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameDecodeError::Empty => {
                f.write_str("volume stat response frame is empty")
            }
            FrameDecodeError::UnknownTag(tag) => {
                write!(f, "unknown volume stat response frame tag {tag}")
            }
            FrameDecodeError::Stat(error) => {
                write!(f, "volume stat did not parse: {error}")
            }
            FrameDecodeError::Error(error) => {
                write!(f, "volume stat error did not parse: {error}")
            }
        }
    }
}

impl std::error::Error for FrameDecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameDecodeError::Stat(error) => Some(error),
            FrameDecodeError::Error(error) => Some(error),
            FrameDecodeError::Empty | FrameDecodeError::UnknownTag(_) => None,
        }
    }
}
