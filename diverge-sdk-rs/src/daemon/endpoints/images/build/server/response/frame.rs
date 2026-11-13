//! What a server's response frame carries for an image build.

use serde::Serialize;
use std::fmt;

use super::{Built, Exists};
use crate::wire::decode::Decode;
use crate::wire::encode::{Encode, Writer};
use crate::shared::error::Error;

/// A line the build said, or the one answer that ends it.
///
/// A build is a stream with an end: zero or more
/// [`Output`](Self::Output) frames, each a line the provider's build
/// tool wrote, as it wrote them; then exactly one of the others; then
/// the finish, and nothing after it. The rules:
///
/// 1. **Judged first.** A request the caller's grants do not allow is
///    answered [`Forbidden`](Self::Forbidden), and a provider the
///    daemon keeps no record of, or a context volume the provider does
///    not list, [`NotFound`](Self::NotFound), with no output before
///    either: nothing was asked of the provider.
/// 2. **Then the provider's own answers, relayed.**
///    [`Built`](Self::Built): the image is in the daemon's Store on
///    the provider. [`Exists`](Self::Exists): the build produced a
///    digest owned there already, and nothing was added.
///    [`InsufficientCapacity`](Self::InsufficientCapacity): the Store
///    has no room. [`ContextHeld`](Self::ContextHeld): something holds
///    the context volume. [`Unsupported`](Self::Unsupported): the
///    provider has no Store. The three refusals come with no output
///    before them, and after each of them nothing exists.
/// 3. **Or a failure.** [`Error`](Self::Error): the provider is not
///    connected, the build failed, or the exchange broke. The output
///    before it says what the build said.
///
/// Each line of output is text for a person: nothing promises what the
/// lines say, how many there are, or that there are any.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Frame {
    /// A line of the build's output, as the build tool wrote it. Tag
    /// `0`.
    Output(String),
    /// The image is in the daemon's Store on the provider. Tag `1`.
    Built(Built),
    /// The build produced a digest owned already, and nothing was
    /// added. Tag `2`.
    Exists(Exists),
    /// The Store has no room for the image, and nothing exists. Tag
    /// `3`.
    InsufficientCapacity,
    /// Something holds the volume the context names — a running
    /// container, or an operation that has it to itself — and nothing
    /// exists. Tag `4`.
    ContextHeld,
    /// The provider has no Store, and nothing exists. Tag `5`.
    Unsupported,
    /// The caller's grants do not allow it: no `build` over images,
    /// or no `mount` over the context volume. Tag `6`.
    Forbidden,
    /// The provider is none the daemon keeps a record of, or the
    /// context volume is none the provider lists. Tag `7`.
    NotFound,
    /// A failure, in the daemon's words or the provider's. Tag `8`.
    Error(Error),
}

/// Tag for [`Frame::Output`].
const OUTPUT: u8 = 0;

/// Tag for [`Frame::Built`].
const BUILT: u8 = 1;

/// Tag for [`Frame::Exists`].
const EXISTS: u8 = 2;

/// Tag for [`Frame::InsufficientCapacity`].
const INSUFFICIENT_CAPACITY: u8 = 3;

/// Tag for [`Frame::ContextHeld`].
const CONTEXT_HELD: u8 = 4;

/// Tag for [`Frame::Unsupported`].
const UNSUPPORTED: u8 = 5;

/// Tag for [`Frame::Forbidden`].
const FORBIDDEN: u8 = 6;

/// Tag for [`Frame::NotFound`].
const NOT_FOUND: u8 = 7;

/// Tag for [`Frame::Error`].
const ERROR: u8 = 8;

/// The tag, then the variant's own JSON; a bare answer carries
/// nothing after its tag.
impl Encode for Frame {
    /// The ordinary JSON failure. The tag cannot fail.
    type Error = serde_json::Error;

    // Spelled out rather than `Self::Error`: this enum has a variant
    // called `Error`, so the associated type is ambiguous by that name.
    fn encode(&self, out: &mut Writer<'_>) -> Result<(), serde_json::Error> {
        match self {
            Frame::Output(line) => {
                out.extend_from_slice(&[OUTPUT]);
                serde_json::to_writer(out, line)
            }
            Frame::Built(built) => {
                out.extend_from_slice(&[BUILT]);
                serde_json::to_writer(out, built)
            }
            Frame::Exists(exists) => {
                out.extend_from_slice(&[EXISTS]);
                serde_json::to_writer(out, exists)
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
            Frame::Forbidden => {
                out.extend_from_slice(&[FORBIDDEN]);
                Ok(())
            }
            Frame::NotFound => {
                out.extend_from_slice(&[NOT_FOUND]);
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
    /// Each failure names which variant's bytes would not parse.
    type Error = FrameError;

    // Spelled out for the same reason as `encode` above.
    fn decode(bytes: &[u8]) -> Result<Self, FrameError> {
        let (tag, rest) = bytes.split_first().ok_or(FrameError::Empty)?;
        match *tag {
            OUTPUT => serde_json::from_slice(rest).map(Frame::Output).map_err(FrameError::Output),
            BUILT => serde_json::from_slice(rest).map(Frame::Built).map_err(FrameError::Built),
            EXISTS => serde_json::from_slice(rest).map(Frame::Exists).map_err(FrameError::Built),
            INSUFFICIENT_CAPACITY => Ok(Frame::InsufficientCapacity),
            CONTEXT_HELD => Ok(Frame::ContextHeld),
            UNSUPPORTED => Ok(Frame::Unsupported),
            FORBIDDEN => Ok(Frame::Forbidden),
            NOT_FOUND => Ok(Frame::NotFound),
            ERROR => Error::decode(rest).map(Frame::Error).map_err(FrameError::Error),
            tag => Err(FrameError::UnknownTag(tag)),
        }
    }
}

/// An images build response frame that could not be read.
#[derive(Debug)]
pub enum FrameError {
    /// No bytes at all, so not even a tag.
    Empty,
    /// A tag that is none of this frame's nine.
    UnknownTag(u8),
    /// The line did not parse.
    Output(serde_json::Error),
    /// The image built, or owned already, did not parse.
    Built(serde_json::Error),
    /// The error did not parse.
    Error(serde_json::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Empty => f.write_str("images build response frame is empty"),
            FrameError::UnknownTag(tag) => write!(f, "unknown images build response frame tag {tag}"),
            FrameError::Output(error) => write!(f, "images build output did not parse: {error}"),
            FrameError::Built(error) => write!(f, "images build result did not parse: {error}"),
            FrameError::Error(error) => write!(f, "images build error did not parse: {error}"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Output(error) | FrameError::Built(error) | FrameError::Error(error) => Some(error),
            FrameError::Empty | FrameError::UnknownTag(_) => None,
        }
    }
}
