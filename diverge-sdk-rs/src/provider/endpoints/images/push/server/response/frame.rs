//! What a server's response frame carries for an image push.

use std::fmt;

use super::{Blob, Exists, Pushed};
use crate::shared::error::Error;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// A blob pulled; the image pushed; the word that the caller owns it
/// already; a refusal; or the news that the provider could not push
/// it.
///
/// A push is a stream with an end. Zero or more [`Blob`](Self::Blob)
/// frames, one per blob pulled from the caller; then exactly one of
/// [`Pushed`](Self::Pushed), [`Exists`](Self::Exists),
/// [`InsufficientCapacity`](Self::InsufficientCapacity),
/// [`Unavailable`](Self::Unavailable),
/// [`ProofFailed`](Self::ProofFailed),
/// [`Malformed`](Self::Malformed), [`Unsupported`](Self::Unsupported)
/// and [`Error`](Self::Error); then the finish, and nothing after it.
/// A payload leads with one byte saying which — `0` for `Blob`, `1`
/// for `Pushed`, `2` for `Exists`, `3` for `InsufficientCapacity`, `4`
/// for `Unavailable`, `5` for `ProofFailed`, `6` for `Malformed`, `7`
/// for `Unsupported`, `8` for `Error` — and the rest is that variant's
/// own bytes; the five bare refusals carry nothing.
///
/// | the scope ends with | means |
/// |----------------------|-------|
/// | [`Pushed`](Self::Pushed), then a finish | the image is in the caller's Store, and a listing will show it |
/// | [`Exists`](Self::Exists), then a finish | the caller owned the digest already; nothing moved |
/// | [`InsufficientCapacity`](Self::InsufficientCapacity), then a finish | the Store has no room for it, by the sizes its manifest declares; nothing moved |
/// | [`Unavailable`](Self::Unavailable), then a finish | no registry the provider uses serves it, and the caller did not supply all of it; nothing is owned |
/// | [`ProofFailed`](Self::ProofFailed), then a finish | the caller supplied the image and did not prove it holds it; nothing is owned, and what the attempt brought in is removed |
/// | [`Malformed`](Self::Malformed), then a finish | a manifest that does not parse, a media type the provider does not take, or a size or a digest that does not match; nothing is owned |
/// | [`Unsupported`](Self::Unsupported), then a finish | the provider has no Store; nothing moved |
/// | an [`Error`](Self::Error), then a finish | it is not owned, for some other reason |
///
/// # The refusals are answers, not errors
///
/// Each says what a caller does next: delete an image and ask again,
/// name a reference or hold the bytes, answer the proof from the
/// image's own bytes, fix the image, or ask another provider. An
/// [`Error`](Self::Error) says none of that.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// A blob pulled from the caller and verified. Tag `0`.
    Blob(Blob),
    /// The image is in the caller's Store. Tag `1`.
    Pushed(Pushed),
    /// The caller already owns the digest, and nothing moved. Tag `2`.
    Exists(Exists),
    /// The Store has no room for the image, by the sizes its manifest
    /// declares, and nothing moved past the manifest. Tag `3`.
    InsufficientCapacity,
    /// No registry the provider uses serves the image, and the caller
    /// did not supply a manifest or a blob of it. Tag `4`.
    Unavailable,
    /// The caller supplied the image and did not prove it holds it —
    /// see [`oci::prove`](crate::shared::containers::oci::prove) — and
    /// what the attempt brought in is removed. Tag `5`.
    ProofFailed,
    /// The image is not one the provider takes: a manifest that does
    /// not parse, a media type it does not take, or a size or a digest
    /// that does not match the bytes. Tag `6`.
    Malformed,
    /// The provider has no Store — its `images_store` is `false`, see
    /// [`info`](crate::provider::endpoints::info) — and nothing moved.
    /// Tag `7`.
    Unsupported,
    /// A failure. Tag `8`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Blob`].
const BLOB: u8 = 0;

/// Tag for [`Frame::Pushed`].
const PUSHED: u8 = 1;

/// Tag for [`Frame::Exists`].
const EXISTS: u8 = 2;

/// Tag for [`Frame::InsufficientCapacity`].
const INSUFFICIENT_CAPACITY: u8 = 3;

/// Tag for [`Frame::Unavailable`].
const UNAVAILABLE: u8 = 4;

/// Tag for [`Frame::ProofFailed`].
const PROOF_FAILED: u8 = 5;

/// Tag for [`Frame::Malformed`].
const MALFORMED: u8 = 6;

/// Tag for [`Frame::Unsupported`].
const UNSUPPORTED: u8 = 7;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 8;

/// Two formats, and the tag chooses between them: a blob, the image
/// pushed and the image owned already are postcard, as the rest of
/// the Store is, and the error is JSON, since a [`serde_json::Value`]
/// cannot come back out of a format with no self-description.
impl Encode for Frame {
    /// One failure per half, and they are different libraries'.
    type Error = FrameEncodeError;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), FrameEncodeError> {
        match self {
            Frame::Blob(blob) => {
                out.extend_from_slice(&[BLOB]);
                postcard::to_io(blob, &mut *out).map(|_| ()).map_err(FrameEncodeError::Body)
            }
            Frame::Pushed(pushed) => {
                out.extend_from_slice(&[PUSHED]);
                postcard::to_io(pushed, &mut *out).map(|_| ()).map_err(FrameEncodeError::Body)
            }
            Frame::Exists(exists) => {
                out.extend_from_slice(&[EXISTS]);
                postcard::to_io(exists, &mut *out).map(|_| ()).map_err(FrameEncodeError::Body)
            }
            Frame::InsufficientCapacity => {
                out.extend_from_slice(&[INSUFFICIENT_CAPACITY]);
                Ok(())
            }
            Frame::Unavailable => {
                out.extend_from_slice(&[UNAVAILABLE]);
                Ok(())
            }
            Frame::ProofFailed => {
                out.extend_from_slice(&[PROOF_FAILED]);
                Ok(())
            }
            Frame::Malformed => {
                out.extend_from_slice(&[MALFORMED]);
                Ok(())
            }
            Frame::Unsupported => {
                out.extend_from_slice(&[UNSUPPORTED]);
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
            BLOB => postcard::from_bytes(rest).map(Frame::Blob).map_err(FrameDecodeError::Body),
            PUSHED => postcard::from_bytes(rest).map(Frame::Pushed).map_err(FrameDecodeError::Body),
            EXISTS => postcard::from_bytes(rest).map(Frame::Exists).map_err(FrameDecodeError::Body),
            INSUFFICIENT_CAPACITY => Ok(Frame::InsufficientCapacity),
            UNAVAILABLE => Ok(Frame::Unavailable),
            PROOF_FAILED => Ok(Frame::ProofFailed),
            MALFORMED => Ok(Frame::Malformed),
            UNSUPPORTED => Ok(Frame::Unsupported),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameDecodeError::Error),
            tag => Err(FrameDecodeError::UnknownTag(tag)),
        }
    }
}

/// An image push response that could not be written.
#[derive(Debug)]
pub enum FrameEncodeError {
    /// A blob, the image pushed or the image owned already did not
    /// serialize.
    Body(postcard::Error),
    /// The error did not serialize.
    Error(serde_json::Error),
}

impl fmt::Display for FrameEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameEncodeError::Body(error) => write!(f, "image push answer did not serialize: {error}"),
            FrameEncodeError::Error(error) => write!(f, "image push error did not serialize: {error}"),
        }
    }
}

impl std::error::Error for FrameEncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameEncodeError::Body(error) => Some(error),
            FrameEncodeError::Error(error) => Some(error),
        }
    }
}

/// An image push response that could not be read.
#[derive(Debug)]
pub enum FrameDecodeError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's nine.
    UnknownTag(u8),
    /// A blob, the image pushed or the image owned already did not
    /// parse.
    Body(postcard::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameDecodeError::Empty => f.write_str("image push response frame is empty"),
            FrameDecodeError::UnknownTag(tag) => write!(f, "unknown image push response frame tag {tag}"),
            FrameDecodeError::Body(error) => write!(f, "image push answer did not parse: {error}"),
            FrameDecodeError::Error(error) => write!(f, "image push error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameDecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameDecodeError::Body(error) => Some(error),
            FrameDecodeError::Error(error) => Some(error),
            FrameDecodeError::Empty | FrameDecodeError::UnknownTag(_) => None,
        }
    }
}
