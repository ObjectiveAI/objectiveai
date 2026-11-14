//! What a server's response frame carries for an image push.

use serde::Serialize;
use std::fmt;

use super::{Blob, Exists, Pushed};
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// A blob pulled, or the one answer that ends the push.
///
/// A push is a stream with an end: zero or more [`Blob`](Self::Blob)
/// frames, one per blob the provider pulled from the client; then
/// exactly one of the others; then the finish, and nothing after it.
/// The rules:
///
/// 1. **Judged first.** A request the caller's grants do not allow is
///    answered [`Forbidden`](Self::Forbidden), and a provider the
///    daemon keeps no record of [`NotFound`](Self::NotFound), with
///    nothing before either: nothing was asked of the provider.
/// 2. **Then the provider's own answers, relayed.**
///    [`Pushed`](Self::Pushed): the image is in the daemon's Store on
///    the provider. [`Exists`](Self::Exists): the daemon owned the
///    digest there already, and nothing moved.
///    [`InsufficientCapacity`](Self::InsufficientCapacity): the Store
///    has no room, by the sizes the manifest declares.
///    [`Unavailable`](Self::Unavailable): no registry the provider
///    uses serves the image, and the client did not supply all of it.
///    [`ProofFailed`](Self::ProofFailed): the client supplied the
///    image and did not prove it holds it.
///    [`Malformed`](Self::Malformed): the image is not one the
///    provider takes. [`Unsupported`](Self::Unsupported): the provider
///    has no Store. After each of the five refusals nothing is owned.
/// 3. **Or a failure.** [`Error`](Self::Error): the provider is not
///    connected, or the exchange broke.
///
/// A payload leads with one byte saying which, and the rest is that
/// variant's own JSON; a bare answer carries nothing after its tag.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Frame {
    /// A blob pulled from the client and verified. Tag `0`.
    Blob(Blob),
    /// The image is in the daemon's Store on the provider. Tag `1`.
    Pushed(Pushed),
    /// The daemon already owns the digest there, and nothing moved.
    /// Tag `2`.
    Exists(Exists),
    /// The Store has no room for the image, and nothing moved past the
    /// manifest. Tag `3`.
    InsufficientCapacity,
    /// No registry the provider uses serves the image, and the client
    /// did not supply a manifest or a blob of it. Tag `4`.
    Unavailable,
    /// The client supplied the image and did not prove it holds it;
    /// what the attempt brought in is removed. Tag `5`.
    ProofFailed,
    /// The image is not one the provider takes: a manifest that does
    /// not parse, a media type it does not take, or a size or a digest
    /// that does not match the bytes. Tag `6`.
    Malformed,
    /// The provider has no Store, and nothing moved. Tag `7`.
    Unsupported,
    /// The caller's grants do not allow it: no `push` over images
    /// reaching the provider. Tag `8`.
    Forbidden,
    /// The provider is none the daemon keeps a record of. Tag `9`.
    NotFound,
    /// A failure, in the daemon's words or the provider's. Tag `10`.
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

/// Tag for [`Frame::Forbidden`].
const FORBIDDEN: u8 = 8;

/// Tag for [`Frame::NotFound`].
const NOT_FOUND: u8 = 9;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 10;

/// The tag, then the variant's own JSON; a bare answer carries
/// nothing after its tag.
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
            Frame::Blob(blob) => {
                out.extend_from_slice(&[BLOB]);
                serde_json::to_writer(out, blob)
            }
            Frame::Pushed(pushed) => {
                out.extend_from_slice(&[PUSHED]);
                serde_json::to_writer(out, pushed)
            }
            Frame::Exists(exists) => {
                out.extend_from_slice(&[EXISTS]);
                serde_json::to_writer(out, exists)
            }
            Frame::InsufficientCapacity => bare(out, INSUFFICIENT_CAPACITY),
            Frame::Unavailable => bare(out, UNAVAILABLE),
            Frame::ProofFailed => bare(out, PROOF_FAILED),
            Frame::Malformed => bare(out, MALFORMED),
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
            BLOB => serde_json::from_slice(rest).map(Frame::Blob).map_err(FrameError::Blob),
            PUSHED => serde_json::from_slice(rest).map(Frame::Pushed).map_err(FrameError::Pushed),
            EXISTS => serde_json::from_slice(rest).map(Frame::Exists).map_err(FrameError::Pushed),
            INSUFFICIENT_CAPACITY => Ok(Frame::InsufficientCapacity),
            UNAVAILABLE => Ok(Frame::Unavailable),
            PROOF_FAILED => Ok(Frame::ProofFailed),
            MALFORMED => Ok(Frame::Malformed),
            UNSUPPORTED => Ok(Frame::Unsupported),
            FORBIDDEN => Ok(Frame::Forbidden),
            NOT_FOUND => Ok(Frame::NotFound),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An images push response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's eleven.
    UnknownTag(u8),
    /// The blob pulled did not parse.
    Blob(serde_json::Error),
    /// The image pushed, or owned already, did not parse.
    Pushed(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("images push response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown images push response frame tag {tag}"),
            FrameError::Blob(error) => write!(f, "images push blob did not parse: {error}"),
            FrameError::Pushed(error) => write!(f, "images push result did not parse: {error}"),
            FrameError::Error(error) => write!(f, "images push error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Blob(error) | FrameError::Pushed(error) | FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
