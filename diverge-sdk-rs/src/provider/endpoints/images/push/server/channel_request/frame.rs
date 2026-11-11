//! What a server's channel request frame carries for an image push.

use std::fmt;

use crate::shared::containers::oci;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// What a provider asks a caller for while it takes a pushed image
/// from it: three of the asks of
/// [`oci`](crate::shared::containers::oci), each with the run's own
/// request body and the run's own answer.
///
/// | tag | asks for |
/// |-----|----------|
/// | `0` | [`OciManifest`](Self::OciManifest) |
/// | `1` | [`OciBlob`](Self::OciBlob) |
/// | `2` | [`OciProve`](Self::OciProve) |
///
/// The manifest and the blob keep the runs' tags, `0` and `1`. The
/// proof takes `2`, the first tag free here: the runs' `2` is the
/// question whether the caller holds the image, which a push never
/// asks — the caller named the image to push — and the runs number
/// the proof after their own asks, at `20` for agents and `16` for
/// tools. Nothing else is asked: the provider opens no channel on a
/// push but these, and only when it takes the image from the caller.
///
/// # In what order
///
/// 1. One [`OciManifest`](Self::OciManifest) for the manifest the
///    request's digest names, and, when that manifest is an index,
///    one for the platform manifest the provider runs.
/// 2. One [`OciBlob`](Self::OciBlob) for every blob that manifest
///    references and the provider does not already hold. None, when
///    it holds them all.
/// 3. Exactly one [`OciProve`](Self::OciProve), always, last, whether
///    or not any blob was asked for.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Frame {
    /// A manifest of the image being pushed, by digest. Tag `0`.
    ///
    /// Answered with at most one frame, the media type and the bytes,
    /// then the finish; the empty finish is a digest the caller does
    /// not hold, and the push is `Unavailable`. See
    /// [`oci::manifest`](crate::shared::containers::oci::manifest).
    OciManifest(oci::manifest::request::Request),
    /// A blob of the image being pushed, by digest. Tag `1`.
    ///
    /// Answered with the blob's bytes as zero or more frames, each
    /// appending, then the finish; zero frames is a digest the caller
    /// does not hold. See
    /// [`oci::blob`](crate::shared::containers::oci::blob).
    OciBlob(oci::blob::request::Request),
    /// The proof that the caller holds the image. Tag `2`.
    ///
    /// Answered with one frame of 32 bytes, or the empty finish for
    /// not held; anything but the provider's own value is the push's
    /// `ProofFailed`. See
    /// [`oci::prove`](crate::shared::containers::oci::prove).
    OciProve(oci::prove::request::Request),
}

/// Tag for [`Frame::OciManifest`].
const OCI_MANIFEST: u8 = 0;

/// Tag for [`Frame::OciBlob`].
const OCI_BLOB: u8 = 1;

/// Tag for [`Frame::OciProve`].
const OCI_PROVE: u8 = 2;

/// A tag, then the ask's own JSON.
impl Encode for Frame {
    /// The ordinary JSON failure. The tag cannot fail.
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
        }
    }
}

impl Decode<'_> for Frame {
    /// Three ways to fail: nothing, a tag that is none of the three,
    /// or an ask that did not parse.
    type Error = FrameError;

    fn decode(bytes: &[u8]) -> Result<Self, Self::Error> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            OCI_MANIFEST => oci::manifest::request::Request::decode(rest).map(Frame::OciManifest).map_err(FrameError::Oci),
            OCI_BLOB => oci::blob::request::Request::decode(rest).map(Frame::OciBlob).map_err(FrameError::Oci),
            OCI_PROVE => oci::prove::request::Request::decode(rest).map(Frame::OciProve).map_err(FrameError::Oci),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An image push channel request that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's three.
    UnknownTag(u8),
    /// An ask did not parse.
    Oci(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("image push channel request frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown image push channel request tag {tag}"),
            FrameError::Oci(error) => write!(f, "image push ask did not parse: {error}"),
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
