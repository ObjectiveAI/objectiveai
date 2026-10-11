//! What a client's channel request frame carries on an image pull.

use std::fmt;

use crate::shared::containers::oci;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// What a caller asks a provider for on an image pull: the three asks
/// of [`oci`](crate::shared::containers::oci), byte for byte as the
/// provider asks them of a caller on a run scope — the same tags, the
/// same request bodies, the same answers — and the disconnect that
/// ends the scope.
///
/// | tag | asks for |
/// |-----|----------|
/// | `0` | [`OciManifest`](Self::OciManifest) |
/// | `1` | [`OciBlob`](Self::OciBlob) |
/// | `2` | [`OciHas`](Self::OciHas) |
/// | `3` | [`Disconnect`](Self::Disconnect) |
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Frame {
    /// A manifest of an image the provider holds for the caller, by
    /// digest. Tag `0`.
    ///
    /// Answered with at most one frame, the media type and the bytes,
    /// then the finish; the empty finish for a digest the provider
    /// does not hold for the caller. See
    /// [`oci::manifest`](crate::shared::containers::oci::manifest).
    OciManifest(oci::manifest::request::Request),
    /// A blob of such an image, by digest. Tag `1`.
    ///
    /// Answered with the blob's bytes as zero or more frames, each
    /// appending, then the finish; zero frames is a digest the
    /// provider does not hold for the caller. See
    /// [`oci::blob`](crate::shared::containers::oci::blob).
    OciBlob(oci::blob::request::Request),
    /// Whether the provider holds an image for the caller, by digest,
    /// and under what name. Tag `2`.
    ///
    /// Answered with one frame, the byte `0` or the byte `1` and a
    /// repository path, then the finish. See
    /// [`oci::has`](crate::shared::containers::oci::has).
    OciHas(oci::has::request::Request),
    /// Leave the scope. Tag `3`.
    ///
    /// Carries nothing — the variant is bare — and nothing comes back
    /// on this channel: the provider sends no channel response and no
    /// channel response finish on it. What comes back is the scope's
    /// finish, after every ask still being answered has been.
    Disconnect,
}

/// Tag for [`Frame::OciManifest`].
const OCI_MANIFEST: u8 = 0;

/// Tag for [`Frame::OciBlob`].
const OCI_BLOB: u8 = 1;

/// Tag for [`Frame::OciHas`].
const OCI_HAS: u8 = 2;

/// Tag for [`Frame::Disconnect`].
const DISCONNECT: u8 = 3;

/// A tag, then the ask's own JSON; the disconnect is the tag alone.
impl Encode for Frame {
    /// The ordinary JSON failure, from the three variants that have
    /// one. A lone tag byte cannot fail.
    type Error = serde_json::Error;

    fn encode(&self, out: &mut Writer<'_>) -> Result<(), Self::Error> {
        match self {
            Frame::OciManifest(request) => {
                out.extend_from_slice(&[OCI_MANIFEST]);
                request.encode(out)
            }
            Frame::OciBlob(request) => {
                out.extend_from_slice(&[OCI_BLOB]);
                request.encode(out)
            }
            Frame::OciHas(request) => {
                out.extend_from_slice(&[OCI_HAS]);
                request.encode(out)
            }
            Frame::Disconnect => {
                out.extend_from_slice(&[DISCONNECT]);
                Ok(())
            }
        }
    }
}

impl Decode<'_> for Frame {
    /// Three ways to fail: nothing, a tag that is none of the four, or
    /// an ask that did not parse.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            OCI_MANIFEST => oci::manifest::request::Request::decode(rest).map(Frame::OciManifest).map_err(FrameError::Oci),
            OCI_BLOB => oci::blob::request::Request::decode(rest).map(Frame::OciBlob).map_err(FrameError::Oci),
            OCI_HAS => oci::has::request::Request::decode(rest).map(Frame::OciHas).map_err(FrameError::Oci),
            DISCONNECT => Ok(Frame::Disconnect),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An image pull channel request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's four.
    UnknownTag(u8),
    /// An ask did not parse.
    Oci(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("image pull channel request frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown image pull channel request tag {tag}"),
            FrameError::Oci(error) => write!(f, "image pull ask did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Oci(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
