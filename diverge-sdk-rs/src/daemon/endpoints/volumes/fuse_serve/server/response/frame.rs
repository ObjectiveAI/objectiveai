//! What a server's response frame carries for a volume FUSE serve.

use serde::Serialize;
use std::fmt;

use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::containers::response::VolumeMode;
use crate::shared::error::Error;

/// Whether the volume is served: exactly one, first on the main
/// stream. After [`Serving`](Self::Serving) the scope stays open and
/// nothing more comes on the main stream until the finish; after any
/// other the finish follows at once, and no channel is answered.
///
/// | the scope answers | means |
/// |-------------------|-------|
/// | [`Serving`](Self::Serving) | the provider serves the volume, and the scope carries its asks |
/// | [`Mounted`](Self::Mounted) | a running container has the volume, and the provider serves no volume of its mode beside one |
/// | [`FuseServed`](Self::FuseServed) | another serve holds the volume, and the provider serves a volume of its mode once |
/// | [`Held`](Self::Held) | another operation has the volume to itself |
/// | [`VolumeMode`](Self::VolumeMode) | the volume is in another mode than the request meant |
/// | [`Unsupported`](Self::Unsupported) | the provider serves no volume of that mode |
/// | [`Forbidden`](Self::Forbidden) | the caller holds no `fuse_serve` grant reaching the volume |
/// | [`NotFound`](Self::NotFound) | the daemon keeps no record of the provider, or the provider lists no such volume |
/// | [`Error`](Self::Error) | a failure, in the daemon's words or the provider's |
///
/// The five refusals after `Serving` are the provider's own, relayed
/// as it gave them: see
/// [`volumes::fuse_serve`](crate::provider::endpoints::volumes::fuse_serve).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Frame {
    /// The volume is served. Tag `0`.
    Serving,
    /// A running container has the volume. Tag `1`.
    Mounted,
    /// Another FUSE serve holds the volume. Tag `2`.
    FuseServed,
    /// Another operation has the volume to itself. Tag `3`.
    Held,
    /// The volume is in another mode: its name, and the mode it is in.
    /// Tag `4`.
    VolumeMode(VolumeMode),
    /// The provider serves no volume of that mode. Tag `5`.
    Unsupported,
    /// The account the request is served for holds no grant allowing
    /// it; nothing was asked of the provider. Tag `6`.
    Forbidden,
    /// The provider is none the daemon keeps a record of, or the volume
    /// is none the provider lists. Tag `7`.
    NotFound,
    /// A failure. Tag `8`.
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

/// Tag for [`Frame::Forbidden`].
const FORBIDDEN: u8 = 6;

/// Tag for [`Frame::NotFound`].
const NOT_FOUND: u8 = 7;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 8;

/// The tag, then the variant's own JSON; a bare answer carries nothing
/// after its tag.
impl Encode for Frame {
    /// The ordinary JSON failure. The tag cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        let bare = |out: &mut Writer<'_>, tag: u8| {
            out.extend_from_slice(&[tag]);
            Ok(())
        };
        match self {
            Frame::Serving => bare(out, SERVING),
            Frame::Mounted => bare(out, MOUNTED),
            Frame::FuseServed => bare(out, FUSE_SERVED),
            Frame::Held => bare(out, HELD),
            Frame::VolumeMode(refused) => {
                out.extend_from_slice(&[VOLUME_MODE]);
                serde_json::to_writer(out, refused)
            }
            Frame::Unsupported => bare(out, UNSUPPORTED),
            Frame::Forbidden => bare(out, FORBIDDEN),
            Frame::NotFound => bare(out, NOT_FOUND),
            Frame::Error(error) => {
                out.extend_from_slice(&[ERROR]);
                error.encode(out)
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Each failure names which variant's bytes would not parse.
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
            FORBIDDEN => Ok(Frame::Forbidden),
            NOT_FOUND => Ok(Frame::NotFound),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// A volumes fuse serve response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's nine.
    UnknownTag(u8),
    /// The mode refusal did not parse.
    VolumeMode(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("volumes fuse serve response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown volumes fuse serve response frame tag {tag}"),
            FrameError::VolumeMode(error) => write!(f, "volumes fuse serve mode refusal did not parse: {error}"),
            FrameError::Error(error) => write!(f, "volumes fuse serve error did not parse: {error}"),
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
