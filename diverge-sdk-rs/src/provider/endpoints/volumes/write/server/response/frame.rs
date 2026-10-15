//! What a server's response frame carries for a volume write.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::containers::write_path;
use crate::shared::error::Error;

/// The file landed, or it did not.
///
/// A payload leads with one byte saying which — `0` for
/// [`Written`](Self::Written), `1` for [`Mounted`](Self::Mounted),
/// `2` for [`FuseServed`](Self::FuseServed), `3` for
/// [`Held`](Self::Held), `4` for [`Unsupported`](Self::Unsupported),
/// `5` for [`Error`](Self::Error) — and the rest is that variant's own
/// bytes, of which the four refusals have none.
///
/// # How it ends
///
/// | the scope ends with | means |
/// |---------------------|-------|
/// | a [`Written`](Self::Written), then a finish | the file is at the path |
/// | [`Mounted`](Self::Mounted), then a finish | a running container has the volume, and nothing was written |
/// | [`FuseServed`](Self::FuseServed), then a finish | a FUSE serve holds the volume, and nothing was written |
/// | [`Held`](Self::Held), then a finish | another operation has the volume to itself, and nothing was written |
/// | an [`Error`](Self::Error), then a finish | it is not, and nothing partial is |
///
/// Only a finish ends it, and it comes only once the content channel
/// has — or at once, for a write refused before the content was asked
/// for: a volume the caller cannot see, one that something holds, a
/// parent that is not a directory.
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
/// # The failure is this scope's, the answer is not
///
/// [`Written`](Self::Written) carries
/// [`write_path::response::Frame`](crate::shared::containers::write_path::response::Frame),
/// which means the same thing wherever the exchange happens. The
/// [`Error`](Self::Error) beside it is the exchange's own.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The file is at the path. Tag `0`.
    Written(write_path::response::Frame),
    /// The volume is mounted in a running container, and was not
    /// written. Tag `1`.
    ///
    /// Nothing was changed; a caller stops the container and asks
    /// again.
    Mounted,
    /// A FUSE serve holds the volume, and it was not written.
    /// Tag `2`.
    ///
    /// Nothing was changed; a caller stops the FUSE serve and asks
    /// again.
    FuseServed,
    /// Another operation — a stat, a read, a write, a filetree, an
    /// edit or a delete — has the volume to itself, and it was not
    /// written. Tag `3`.
    ///
    /// Nothing was changed; a caller waits for it to end and asks
    /// again.
    Held,
    /// The provider keeps no volumes: no mode is among its capabilities,
    /// see [`info`](crate::provider::endpoints::info), and every volume
    /// endpoint answers this. Tag `4`.
    ///
    /// Carries nothing. Nothing was looked at; a caller asks another
    /// provider.
    Unsupported,
    /// A failure. Tag `5`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Written`].
const WRITTEN: u8 = 0;

/// Tag for [`Frame::Mounted`].
const MOUNTED: u8 = 1;

/// Tag for [`Frame::FuseServed`].
const FUSE_SERVED: u8 = 2;

/// Tag for [`Frame::Held`].
const HELD: u8 = 3;

/// Tag for [`Frame::Unsupported`].
const UNSUPPORTED: u8 = 4;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 5;

impl Encode for Frame {
    /// The ordinary JSON failure, from the half that has one. The
    /// other cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(
        &self,
        out: &mut Writer<'_>,
    ) -> Result<(), serde_json::Error> {
        match self {
            Frame::Written(inner) => {
                out.extend_from_slice(&[WRITTEN]);
                // Its error is `Infallible`, and an empty match on one
                // is how you say so: there is no value to handle.
                inner.encode(out).map_err(|error| match error {})
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
            Frame::Unsupported => {
                out.extend_from_slice(&[UNSUPPORTED]);
                Ok(())
            }
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out)
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Three ways to fail, and only one of them is a parse.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            WRITTEN => write_path::response::Frame::decode(rest)
                .map(Frame::Written)
                .map_err(|error| match error {}),
            MOUNTED => Ok(Frame::Mounted),
            FUSE_SERVED => Ok(Frame::FuseServed),
            HELD => Ok(Frame::Held),
            UNSUPPORTED => Ok(Frame::Unsupported),
            ERROR => {
                Error::decode(rest).map(Frame::Error).map_err(FrameError::Error)
            }
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A volume write response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's six.
    UnknownTag(u8),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("volume write response frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown volume write response frame tag {tag}")
            }
            FrameError::Error(error) => {
                write!(f, "volume write error did not parse: {error}")
            }
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
