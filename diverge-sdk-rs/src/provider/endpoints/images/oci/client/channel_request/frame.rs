//! What a client's channel request frame carries on an oci scope.

use std::fmt;

use crate::shared::containers::oci;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// What a caller asks a provider for on an oci scope the provider
/// answered held: three of the asks of
/// [`oci`](crate::shared::containers::oci) — the same request bodies
/// and the same answers as on a run, each over the scope's image —
/// and the cancel that ends the scope.
///
/// | tag | asks for |
/// |-----|----------|
/// | `0` | [`OciManifest`](Self::OciManifest) |
/// | `1` | [`OciBlob`](Self::OciBlob) |
/// | `2` | [`OciProve`](Self::OciProve) |
/// | `3` | [`Cancel`](Self::Cancel) |
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Frame {
    /// A manifest the scope's image reaches, by digest. Tag `0`.
    ///
    /// Answered with at most one frame, the media type and the bytes,
    /// then the finish; the empty finish for a digest the image does
    /// not reach. See
    /// [`oci::manifest`](crate::shared::containers::oci::manifest).
    OciManifest(oci::manifest::request::Request),
    /// A blob the scope's image reaches, by digest. Tag `1`.
    ///
    /// Answered with the blob's bytes as zero or more frames, each
    /// appending, then the finish; zero frames is a digest the image
    /// does not reach. See
    /// [`oci::blob`](crate::shared::containers::oci::blob).
    OciBlob(oci::blob::request::Request),
    /// The proof over the scope's image, under the ask's platform and
    /// nonce. Tag `2`.
    ///
    /// Answered with at most one frame, the 32 bytes of the proof,
    /// then the finish; the empty finish for a platform that breaks
    /// the rule of
    /// [`oci::prove`](crate::shared::containers::oci::prove). See
    /// there.
    OciProve(oci::prove::request::Request),
    /// End the scope. Tag `3`.
    ///
    /// Carries nothing — the variant is bare — and nothing comes back
    /// on this channel: the provider sends no channel response and no
    /// channel response finish on it. What comes back is the scope's
    /// finish, after every ask still being answered has been.
    Cancel,
}

/// Tag for [`Frame::OciManifest`].
const OCI_MANIFEST: u8 = 0;

/// Tag for [`Frame::OciBlob`].
const OCI_BLOB: u8 = 1;

/// Tag for [`Frame::OciProve`].
const OCI_PROVE: u8 = 2;

/// Tag for [`Frame::Cancel`].
const CANCEL: u8 = 3;

/// A tag, then the ask's own JSON; the cancel is the tag alone.
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
            Frame::OciProve(request) => {
                out.extend_from_slice(&[OCI_PROVE]);
                request.encode(out)
            }
            Frame::Cancel => {
                out.extend_from_slice(&[CANCEL]);
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
            OCI_PROVE => oci::prove::request::Request::decode(rest).map(Frame::OciProve).map_err(FrameError::Oci),
            CANCEL => Ok(Frame::Cancel),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An oci scope's channel request that could not be read.
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
            FrameError::Empty => f.write_str("image oci channel request frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown image oci channel request tag {tag}"),
            FrameError::Oci(error) => write!(f, "image oci ask did not parse: {error}"),
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
