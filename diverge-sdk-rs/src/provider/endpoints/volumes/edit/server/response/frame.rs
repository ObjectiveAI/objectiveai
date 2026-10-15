//! What a server's response frame carries for a volume edit.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// The volume reserves what was asked for, or one of two reasons it
/// does not, or it does not for some other reason.
///
/// One of these on channel `0`, then the scope finishes. A payload
/// leads with one byte saying which — `0` for [`Edited`](Self::Edited),
/// `1` for [`InsufficientCapacity`](Self::InsufficientCapacity), `2`
/// for [`ContentTooLarge`](Self::ContentTooLarge), `3` for
/// [`ModeUnsupported`](Self::ModeUnsupported), `4` for
/// [`Mounted`](Self::Mounted), `5` for [`FuseServed`](Self::FuseServed),
/// `6` for [`Held`](Self::Held), `7` for
/// [`Unsupported`](Self::Unsupported), `8` for [`Error`](Self::Error)
/// — and for all but the last there is nothing after it, because
/// saying so IS the whole message.
///
/// | the scope ends with | means |
/// |----------------------|-------|
/// | [`Edited`](Self::Edited), then a finish | a listing will report the new size |
/// | [`InsufficientCapacity`](Self::InsufficientCapacity), then a finish | the provider cannot reserve that many bytes; a listing will report the old size |
/// | [`ContentTooLarge`](Self::ContentTooLarge), then a finish | the volume holds more than that; a listing will report the old size |
/// | [`ModeUnsupported`](Self::ModeUnsupported), then a finish | the provider has no volumes of the mode asked for; the volume is as it was |
/// | [`Mounted`](Self::Mounted), then a finish | a running container has the volume, and the volume is as it was |
/// | [`FuseServed`](Self::FuseServed), then a finish | a FUSE serve holds the volume, and the volume is as it was |
/// | [`Held`](Self::Held), then a finish | another operation has the volume to itself, and the volume is as it was |
/// | an [`Error`](Self::Error), then a finish | a listing will report the old one, for some other reason |
///
/// # The refusals are answers, not errors
///
/// A provider that cannot reserve the size asked for says so with
/// [`InsufficientCapacity`](Self::InsufficientCapacity), and one asked
/// to shrink a volume below what it holds says so with
/// [`ContentTooLarge`](Self::ContentTooLarge) — never with an
/// [`Error`](Self::Error) a caller could not tell from any other
/// failure. The distinction is what a caller acts on: the first is a
/// size to ask smaller, the second a volume to empty first, and a
/// failure is neither. A mode the provider's
/// [`info`](crate::provider::endpoints::info) holds out no capability
/// for is [`ModeUnsupported`](Self::ModeUnsupported): another mode,
/// or another provider. A volume that something holds — a running
/// container, a FUSE serve, another operation — is
/// [`Mounted`](Self::Mounted), [`FuseServed`](Self::FuseServed) or
/// [`Held`](Self::Held), each a thing to stop or wait for. In every
/// refusal nothing changes.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The size is what was asked for. Tag `0`.
    Edited,
    /// The provider cannot reserve that many bytes, and the size is as
    /// it was. Tag `1`.
    InsufficientCapacity,
    /// The volume holds more than the size asked for, so it cannot be
    /// shrunk to it, and the size is as it was. Tag `2`.
    ContentTooLarge,
    /// The mode asked for — by [`Mode`](super::super::super::super::Mode) or
    /// [`Both`](super::super::super::client::request::Change::Both) — is one the
    /// provider has no volumes of, and the volume is as it was. Tag `3`.
    ModeUnsupported,
    /// The volume is mounted in a running container, and was not
    /// edited. Tag `4`.
    ///
    /// Nothing was changed; a caller stops the container and asks
    /// again.
    Mounted,
    /// A FUSE serve holds the volume, and it was not edited.
    /// Tag `5`.
    ///
    /// Nothing was changed; a caller stops the FUSE serve and asks
    /// again.
    FuseServed,
    /// Another operation — a stat, a read, a write, a filetree, an
    /// edit or a delete — has the volume to itself, and it was not
    /// edited. Tag `6`.
    ///
    /// Nothing was changed; a caller waits for it to end and asks
    /// again.
    Held,
    /// The provider keeps no volumes: no mode is among its capabilities,
    /// see [`info`](crate::provider::endpoints::info), and every volume
    /// endpoint answers this. Tag `7`.
    ///
    /// Carries nothing. Nothing was looked at; a caller asks another
    /// provider.
    Unsupported,
    /// A failure. Tag `8`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Edited`].
const EDITED: u8 = 0;

/// Tag for [`Frame::InsufficientCapacity`].
const INSUFFICIENT_CAPACITY: u8 = 1;

/// Tag for [`Frame::ContentTooLarge`].
const CONTENT_TOO_LARGE: u8 = 2;

/// Tag for [`Frame::ModeUnsupported`].
const MODE_UNSUPPORTED: u8 = 3;

/// Tag for [`Frame::Mounted`].
const MOUNTED: u8 = 4;

/// Tag for [`Frame::FuseServed`].
const FUSE_SERVED: u8 = 5;

/// Tag for [`Frame::Held`].
const HELD: u8 = 6;

/// Tag for [`Frame::Unsupported`].
const UNSUPPORTED: u8 = 7;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 8;

/// A tag, and for a failure the JSON after it. Postcard encodes the
/// rest of [`volumes`](crate::provider::endpoints::volumes) and encodes nothing
/// here — an [`Error`](Frame::Error) is a
/// [`serde_json::Value`], which deserializes through
/// `deserialize_any` and so cannot come back out of a format with no
/// self-description. The tag chooses the format, one variant at a
/// time.
impl Encode for Frame {
    /// The ordinary JSON failure, from the one variant that has one.
    /// A lone tag byte cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(
        &self,
        out: &mut Writer<'_>,
    ) -> Result<(), serde_json::Error> {
        match self {
            Frame::Edited => {
                out.extend_from_slice(&[EDITED]);
                Ok(())
            }
            Frame::InsufficientCapacity => {
                out.extend_from_slice(&[INSUFFICIENT_CAPACITY]);
                Ok(())
            }
            Frame::ContentTooLarge => {
                out.extend_from_slice(&[CONTENT_TOO_LARGE]);
                Ok(())
            }
            Frame::ModeUnsupported => {
                out.extend_from_slice(&[MODE_UNSUPPORTED]);
                Ok(())
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
            EDITED => Ok(Frame::Edited),
            INSUFFICIENT_CAPACITY => Ok(Frame::InsufficientCapacity),
            CONTENT_TOO_LARGE => Ok(Frame::ContentTooLarge),
            MODE_UNSUPPORTED => Ok(Frame::ModeUnsupported),
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

/// A volume edit result that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's nine.
    UnknownTag(u8),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => {
                f.write_str("volume edit result frame is empty")
            }
            FrameError::UnknownTag(tag) => {
                write!(f, "unknown volume edit result frame tag {tag}")
            }
            FrameError::Error(error) => {
                write!(f, "volume edit error did not parse: {error}")
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
