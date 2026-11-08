//! What a server's response frame carries for an image hash.

use std::fmt;

use crate::shared::error::Error;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};

/// The proof, or the word that the caller owns no such image, or that
/// the provider has no Store, or that it could not.
///
/// One of these on channel `0`, then the scope finishes. A payload
/// leads with one byte saying which — `0` for [`Hash`](Self::Hash),
/// `1` for [`NotFound`](Self::NotFound), `2` for
/// [`Unsupported`](Self::Unsupported), `3` for [`Error`](Self::Error)
/// — and the rest is that variant's own bytes: the 32 bytes of the
/// proof verbatim for `Hash`, JSON for `Error`, and nothing for the
/// other two.
///
/// | the scope ends with | means |
/// |----------------------|-------|
/// | [`Hash`](Self::Hash), then a finish | the proof over the caller's image, as the provider holds it |
/// | [`NotFound`](Self::NotFound), then a finish | the caller owns no image by the digest, or the platform is not in it |
/// | [`Unsupported`](Self::Unsupported), then a finish | the provider has no Store |
/// | an [`Error`](Self::Error), then a finish | it could not, for some other reason |
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// The proof: `SHA-256(nonce ‖ D)` over the provider's own copy of
    /// the caller's image. Tag `0`, then the 32 bytes.
    Hash {
        /// The proof.
        proof: [u8; 32],
    },
    /// The caller owns no image by the digest, or the platform is not
    /// a manifest of the index it names. Tag `1`.
    NotFound,
    /// The provider has no Store — its `images_store` is `false`, see
    /// [`info`](crate::provider::endpoints::info). Tag `2`.
    ///
    /// Carries nothing. A caller asks another provider; nothing about
    /// the request is the cause.
    Unsupported,
    /// A failure. Tag `3`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Hash`].
const HASH: u8 = 0;

/// Tag for [`Frame::NotFound`].
const NOT_FOUND: u8 = 1;

/// Tag for [`Frame::Unsupported`].
const UNSUPPORTED: u8 = 2;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 3;

/// A tag, then the proof's bytes, the error's JSON, or nothing.
impl Encode for Frame {
    /// The ordinary JSON failure, from the one variant that has one.
    /// The tag and the proof cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Hash { proof } => {
                out.extend_from_slice(&[HASH]);
                out.extend_from_slice(proof);
                Ok(())
            }
            Frame::NotFound => {
                out.extend_from_slice(&[NOT_FOUND]);
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
    /// Four ways to fail, and only one of them is a parse.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            HASH => <[u8; 32]>::try_from(rest)
                .map(|proof| Frame::Hash { proof })
                .map_err(|_| FrameError::Proof(rest.len())),
            NOT_FOUND => Ok(Frame::NotFound),
            UNSUPPORTED => Ok(Frame::Unsupported),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An image hash result that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's four.
    UnknownTag(u8),
    /// A proof of this many bytes, which is not 32.
    Proof(usize),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("image hash result frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown image hash result frame tag {tag}"),
            FrameError::Proof(length) => write!(f, "image hash proof is {length} bytes, not 32"),
            FrameError::Error(error) => write!(f, "image hash error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) | FrameError::Proof(_) => None,
        }
    }
}
