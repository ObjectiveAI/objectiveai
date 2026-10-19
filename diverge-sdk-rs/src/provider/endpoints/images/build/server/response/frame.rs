//! What a server's response frame carries for an image build.

use std::fmt;

use super::Built;
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// A line the build said; the image built; the word that the store
/// has no room for it; or the news that the provider could not build
/// it.
///
/// A build is a stream with an end. Zero or more
/// [`Output`](Self::Output) frames, each a line the provider's build
/// tool wrote, as it wrote them; then exactly one of
/// [`Built`](Self::Built), [`InsufficientCapacity`](Self::InsufficientCapacity),
/// [`ContextHeld`](Self::ContextHeld), [`Unsupported`](Self::Unsupported)
/// and [`Error`](Self::Error); then the finish, and nothing after it.
/// A payload leads with one byte saying which — `0` for `Output`,
/// `1` for `Built`, `2` for `InsufficientCapacity`, `3` for
/// `ContextHeld`, `4` for `Unsupported`, `5` for `Error` — and the
/// rest is that variant's own bytes; the three bare refusals carry
/// nothing, and come with no output before them.
///
/// | the scope ends with | means |
/// |----------------------|-------|
/// | [`Built`](Self::Built), then a finish | the image exists under the name, and a listing will show it |
/// | [`InsufficientCapacity`](Self::InsufficientCapacity), then a finish | the store has no room for it, and nothing exists |
/// | [`ContextHeld`](Self::ContextHeld), then a finish | something holds the context volume, and nothing exists |
/// | [`Unsupported`](Self::Unsupported), then a finish | the provider has no image store, and nothing exists |
/// | an [`Error`](Self::Error), then a finish | it does not exist, for some other reason — a build that failed, a name taken, a context refused — and the output before it says what the build said |
///
/// # The refusals are answers, not errors
///
/// A provider whose store has no room for the image says so with
/// [`InsufficientCapacity`](Self::InsufficientCapacity) rather than
/// with an [`Error`](Self::Error) a caller could not tell from any
/// other failure. The distinction is what a caller acts on: a store
/// with no room is one to delete an image from and ask again, and a
/// failure is not. Whether the provider knows before it builds or
/// only after is the provider's; either way nothing exists. A context
/// volume that something holds — a running container, a FUSE serve,
/// or an operation that has it to itself — is
/// [`ContextHeld`](Self::ContextHeld), a thing to stop or wait for;
/// a provider with no image store at all is
/// [`Unsupported`](Self::Unsupported), another provider to ask.
///
/// # Output is text
///
/// Each line is a string, and the provider decodes what its tool
/// wrote before sending it: a caller shows it to a person and does
/// not parse it. Nothing promises what the lines say, how many there
/// are, or that there are any.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// A line of the build's output, as the build tool wrote it. Tag
    /// `0`.
    Output(String),
    /// The image exists. Tag `1`.
    Built(Built),
    /// The store has no room for the image, and nothing exists. Tag
    /// `2`.
    InsufficientCapacity,
    /// Something holds the volume the context names — a running
    /// container, a FUSE serve, or a stat, a read, a write, a filetree,
    /// an edit or a delete that has it to itself — and nothing exists.
    /// Tag `3`.
    ///
    /// Carries nothing. A build takes the shared hold a run takes,
    /// and a volume that will not take it is this; a caller stops the
    /// holder, or waits for it, and asks again.
    ContextHeld,
    /// The provider has no image store — its `images_store` is `false`,
    /// see [`info`](crate::provider::endpoints::info) — and nothing
    /// exists. Tag `4`.
    ///
    /// Carries nothing. A caller asks another provider; nothing about
    /// the request is the cause.
    Unsupported,
    /// A failure. Tag `5`.
    ///
    /// See [`shared::error::Error`](crate::shared::error::Error) for
    /// why it says so little; the output before it says more.
    Error(Error),
}

/// Tag for [`Frame::Output`].
const OUTPUT: u8 = 0;

/// Tag for [`Frame::Built`].
const BUILT: u8 = 1;

/// Tag for [`Frame::InsufficientCapacity`].
const INSUFFICIENT_CAPACITY: u8 = 2;

/// Tag for [`Frame::ContextHeld`].
const CONTEXT_HELD: u8 = 3;

/// Tag for [`Frame::Unsupported`].
const UNSUPPORTED: u8 = 4;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 5;

/// Two formats, and the tag chooses between them: a line and the
/// image built are postcard, as the rest of the store is, and the
/// error is JSON, since a [`serde_json::Value`] cannot come back out
/// of a format with no self-description.
impl Encode for Frame {
    /// One failure per half, and they are different libraries'.
    type Error = FrameEncodeError;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), FrameEncodeError> {
        match self {
            Frame::Output(line) => {
                out.extend_from_slice(&[OUTPUT]);
                postcard::to_io(line, &mut *out).map(|_| ()).map_err(FrameEncodeError::Output)
            }
            Frame::Built(built) => {
                out.extend_from_slice(&[BUILT]);
                postcard::to_io(built, &mut *out).map(|_| ()).map_err(FrameEncodeError::Built)
            }
            Frame::InsufficientCapacity => {
                out.extend_from_slice(&[INSUFFICIENT_CAPACITY]);
                Ok(())
            }
            Frame::ContextHeld => {
                out.extend_from_slice(&[CONTEXT_HELD]);
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
    /// Five ways to fail, and each names which half failed.
    type Error = FrameDecodeError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameDecodeError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameDecodeError::Empty)?;
        match *tag {
            OUTPUT => postcard::from_bytes(rest).map(Frame::Output).map_err(FrameDecodeError::Output),
            BUILT => postcard::from_bytes(rest).map(Frame::Built).map_err(FrameDecodeError::Built),
            INSUFFICIENT_CAPACITY => Ok(Frame::InsufficientCapacity),
            CONTEXT_HELD => Ok(Frame::ContextHeld),
            UNSUPPORTED => Ok(Frame::Unsupported),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameDecodeError::Error),
            tag => Err(FrameDecodeError::UnknownTag(tag)),
        }
    }
}

/// An image build response that could not be written.
#[derive(Debug)]
pub enum FrameEncodeError {
    /// The line did not serialize.
    Output(postcard::Error),
    /// The image built did not serialize.
    Built(postcard::Error),
    /// The error did not serialize.
    Error(serde_json::Error),
}

impl fmt::Display for FrameEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameEncodeError::Output(error) => write!(f, "image build output did not serialize: {error}"),
            FrameEncodeError::Built(error) => write!(f, "image built did not serialize: {error}"),
            FrameEncodeError::Error(error) => write!(f, "image build error did not serialize: {error}"),
        }
    }
}

impl std::error::Error for FrameEncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameEncodeError::Output(error) => Some(error),
            FrameEncodeError::Built(error) => Some(error),
            FrameEncodeError::Error(error) => Some(error),
        }
    }
}

/// An image build response that could not be read.
#[derive(Debug)]
pub enum FrameDecodeError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's six.
    UnknownTag(u8),
    /// The line did not parse.
    Output(postcard::Error),
    /// The image built did not parse.
    Built(postcard::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameDecodeError::Empty => f.write_str("image build response frame is empty"),
            FrameDecodeError::UnknownTag(tag) => write!(f, "unknown image build response frame tag {tag}"),
            FrameDecodeError::Output(error) => write!(f, "image build output did not parse: {error}"),
            FrameDecodeError::Built(error) => write!(f, "image built did not parse: {error}"),
            FrameDecodeError::Error(error) => write!(f, "image build error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameDecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameDecodeError::Output(error) => Some(error),
            FrameDecodeError::Built(error) => Some(error),
            FrameDecodeError::Error(error) => Some(error),
            FrameDecodeError::Empty | FrameDecodeError::UnknownTag(_) => None,
        }
    }
}
