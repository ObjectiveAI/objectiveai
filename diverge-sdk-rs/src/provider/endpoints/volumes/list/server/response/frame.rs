//! What a server's response frame carries for a volume listing.

use std::fmt;

use super::Volume;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// A volume the caller has, as it comes to be listed, changes, or
/// goes; the word that the listing is whole; or the news that the
/// provider could not say.
///
/// A listing is a stream kept open. First every volume the caller
/// has, each as [`Added`](Self::Added); then exactly one
/// [`Listed`](Self::Listed); then, for the scope's life, a volume the
/// caller creates as `Added`, one it edits as
/// [`Changed`](Self::Changed), one it deletes as
/// [`Removed`](Self::Removed). The finish follows the caller's stop,
/// the caller's connection ending, or exactly one
/// [`Error`](Self::Error), and nothing else: a caller with no volume
/// is listed as `Listed` at once, and watched. A payload leads with
/// one byte saying which — `0` for `Added`, `1` for `Changed`, `2`
/// for `Removed`, `3` for `Listed`, `4` for `Error` — and the rest is
/// that variant's own bytes; `Listed` carries nothing.
///
/// # A finish with nothing is an answer
///
/// The caller stopped before anything was sent, or the request was
/// not served: a scope that finishes with no response before it is
/// that, not a failure. An [`Error`](Self::Error) is a failure: the
/// provider could not tell — a caller that confuses the two stops
/// asking when it should retry.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// A volume the caller has, listed. Tag `0`.
    ///
    /// Once per name while the volume is listed: at the opening for
    /// every volume the caller has then, and after it for one the
    /// caller creates. Nothing promises an order and nothing should
    /// be read into one.
    Added(Volume),
    /// A volume listed already, whose `bytes` or `mode` changed. Tag
    /// `1`.
    ///
    /// The volume as it is now. Its `name` and its `created` are what
    /// they were: neither changes.
    Changed(Volume),
    /// A volume listed already, gone. Tag `2`.
    ///
    /// The volume as it was last listed.
    Removed(Volume),
    /// Every volume the caller had when the scope opened has been
    /// listed. Tag `3`.
    ///
    /// Carries nothing — the variant is bare. A caller that wants the
    /// listing once stops here; one that watches reads on.
    Listed,
    /// A failure. Tag `4`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little.
    Error(Error),
}

/// Tag for [`Frame::Added`].
const ADDED: u8 = 0;

/// Tag for [`Frame::Changed`].
const CHANGED: u8 = 1;

/// Tag for [`Frame::Removed`].
const REMOVED: u8 = 2;

/// Tag for [`Frame::Listed`].
const LISTED: u8 = 3;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 4;

/// Two formats, and the tag chooses between them.
///
/// A volume is **postcard**, matching
/// [`filetree`](crate::shared::filetree) rather than the JSON the rest
/// of the crate uses: this relays nothing, so no byte of it has to
/// survive a round trip unchanged, and nothing downstream reads it as
/// text.
///
/// The error is JSON, and has to be. A
/// [`serde_json::Value`] deserializes through `deserialize_any`, which
/// a format with no self-description cannot answer — so postcard can
/// carry a volume and cannot carry the failure, and each variant gets
/// the format it needs.
impl Encode for Frame {
    /// One failure per half, and they are different libraries'.
    type Error = FrameEncodeError;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), FrameEncodeError> {
        match self {
            Frame::Added(volume) => {
                out.extend_from_slice(&[ADDED]);
                postcard::to_io(volume, &mut *out).map(|_| ()).map_err(FrameEncodeError::Volume)
            }
            Frame::Changed(volume) => {
                out.extend_from_slice(&[CHANGED]);
                postcard::to_io(volume, &mut *out).map(|_| ()).map_err(FrameEncodeError::Volume)
            }
            Frame::Removed(volume) => {
                out.extend_from_slice(&[REMOVED]);
                postcard::to_io(volume, &mut *out).map(|_| ()).map_err(FrameEncodeError::Volume)
            }
            Frame::Listed => {
                out.extend_from_slice(&[LISTED]);
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
            ADDED => postcard::from_bytes(rest).map(Frame::Added).map_err(FrameDecodeError::Volume),
            CHANGED => postcard::from_bytes(rest).map(Frame::Changed).map_err(FrameDecodeError::Volume),
            REMOVED => postcard::from_bytes(rest).map(Frame::Removed).map_err(FrameDecodeError::Volume),
            LISTED => Ok(Frame::Listed),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameDecodeError::Error),
            tag => Err(FrameDecodeError::UnknownTag(tag)),
        }
    }
}

/// A volume listing response that could not be written.
#[derive(Debug)]
pub enum FrameEncodeError {
    /// The volume did not serialize.
    Volume(postcard::Error),
    /// The error did not serialize.
    Error(serde_json::Error),
}

impl fmt::Display for FrameEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameEncodeError::Volume(error) => write!(f, "volume listing volume did not serialize: {error}"),
            FrameEncodeError::Error(error) => write!(f, "volume listing error did not serialize: {error}"),
        }
    }
}

impl std::error::Error for FrameEncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameEncodeError::Volume(error) => Some(error),
            FrameEncodeError::Error(error) => Some(error),
        }
    }
}

/// A volume listing response that could not be read.
#[derive(Debug)]
pub enum FrameDecodeError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's five.
    UnknownTag(u8),
    /// The volume did not parse.
    Volume(postcard::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameDecodeError::Empty => f.write_str("volume listing response frame is empty"),
            FrameDecodeError::UnknownTag(tag) => write!(f, "unknown volume listing response frame tag {tag}"),
            FrameDecodeError::Volume(error) => write!(f, "volume listing volume did not parse: {error}"),
            FrameDecodeError::Error(error) => write!(f, "volume listing error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameDecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameDecodeError::Volume(error) => Some(error),
            FrameDecodeError::Error(error) => Some(error),
            FrameDecodeError::Empty | FrameDecodeError::UnknownTag(_) => None,
        }
    }
}
