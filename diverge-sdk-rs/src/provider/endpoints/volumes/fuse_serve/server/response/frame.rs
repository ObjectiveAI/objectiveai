//! What a server's response frame carries for a volume FUSE serve.

use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::containers::response::VolumeMode;
use crate::shared::error::Error;

/// The volume is served, or the news that it will not be.
///
/// A payload leads with one byte saying which — `0` for
/// [`Serving`](Self::Serving), `1` for [`Mounted`](Self::Mounted),
/// `2` for [`FuseServed`](Self::FuseServed), `3` for
/// [`Held`](Self::Held), `4` for [`VolumeMode`](Self::VolumeMode),
/// `5` for [`Unsupported`](Self::Unsupported), `6` for
/// [`Error`](Self::Error) — and the rest is that variant's own JSON,
/// of which only the mode refusal and the error have any.
///
/// # How the scope runs
///
/// | the scope | means |
/// |-----------|-------|
/// | a serving, then quiet, and stays open | the volume is held, and every ask the caller opens is answered on its channel |
/// | a serving, then a finish | the caller stopped, and the volume is given back |
/// | a mounted, then a finish | it was not served: a running container has the volume, and the provider does not FUSE serve a volume of its mode beside one |
/// | a FUSE served, then a finish | it was not served: another FUSE serve holds the volume, and the provider FUSE serves a volume of its mode once |
/// | a held, then a finish | it was not served: another operation has the volume to itself |
/// | a volume mode, then a finish | it was not served: the volume is not in the mode the request means |
/// | an unsupported, then a finish | it was not served: the provider does not FUSE serve a volume of its mode |
/// | an error, then a finish | it was not served: no volume of the caller's has the name |
///
/// Exactly one response, first. Nothing else comes on channel `0`;
/// the answers to asks come on the asks' channels.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The volume is held, and asks may be opened. Tag `0`.
    Serving,
    /// A running container has the volume, and the provider does not
    /// FUSE serve a volume of its mode beside one — its
    /// `mount_and_fuse_serve` for the mode is `false`. Tag `1`.
    ///
    /// Nothing is held and nothing changed; a caller stops the
    /// container, or asks another provider, and asks again.
    Mounted,
    /// Another FUSE serve holds the volume, and the provider FUSE
    /// serves a volume of its mode once — its `fuse_serve_many` for
    /// the mode is `false`. Tag `2`.
    ///
    /// Nothing is held and nothing changed; a caller stops that FUSE
    /// serve, or asks another provider, and asks again.
    FuseServed,
    /// Another operation — a stat, a read, a write, a filetree, an
    /// edit or a delete — has the volume to itself. Tag `3`.
    ///
    /// Nothing is held and nothing changed; a caller waits for it to
    /// end and asks again.
    Held,
    /// The volume is not in the mode the request means it to have.
    /// Tag `4`.
    ///
    /// Nothing is held and nothing changed; the volume is in the mode
    /// reported. See [`VolumeMode`].
    VolumeMode(VolumeMode),
    /// The provider does not FUSE serve a volume of the mode the
    /// volume is in — its `fuse_serve` for the mode is `false`, see
    /// [`info`](crate::provider::endpoints::info). Tag `5`.
    ///
    /// Nothing is held and nothing changed; a caller asks another
    /// provider, or has the volume edited into a mode this one
    /// serves.
    Unsupported,
    /// A failure. Tag `6`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Serving`].
const SERVING: u8 = 0;

/// Tag for [`Frame::Mounted`].
const MOUNTED: u8 = 1;

/// Tag for [`Frame::FuseServed`].
const FUSE_SERVED: u8 = 2;

/// Tag for [`Frame::Held`].
const HELD: u8 = 3;

/// Tag for [`Frame::VolumeMode`].
const VOLUME_MODE: u8 = 4;

/// Tag for [`Frame::Unsupported`].
const UNSUPPORTED: u8 = 5;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 6;

impl Encode for Frame {
    /// The ordinary JSON failure, from the half that has one.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Serving => {
                out.extend_from_slice(&[SERVING]);
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
            Frame::VolumeMode(refused) => {
                out.extend_from_slice(&[VOLUME_MODE]);
                serde_json::to_writer(out, refused)
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
    /// Four ways to fail, and two of them are parses.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            SERVING => Ok(Frame::Serving),
            MOUNTED => Ok(Frame::Mounted),
            FUSE_SERVED => Ok(Frame::FuseServed),
            HELD => Ok(Frame::Held),
            VOLUME_MODE => serde_json::from_slice(rest).map(Frame::VolumeMode).map_err(FrameError::VolumeMode),
            UNSUPPORTED => Ok(Frame::Unsupported),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A volume FUSE serve response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's seven.
    UnknownTag(u8),
    /// The mode refusal did not parse.
    VolumeMode(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("volume FUSE serve response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown volume FUSE serve response frame tag {tag}"),
            FrameError::VolumeMode(error) => write!(f, "volume FUSE serve mode refusal did not parse: {error}"),
            FrameError::Error(error) => write!(f, "volume FUSE serve error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::VolumeMode(error) | FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
